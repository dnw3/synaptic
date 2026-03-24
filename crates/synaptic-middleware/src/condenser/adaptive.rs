use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::token_estimation::{estimate_message_tokens, estimate_messages, estimate_text};
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use tracing::{debug, warn};

use super::{CondenseAction, CondenseContext, CondenseResult, Condenser};

/// Options for the [`AdaptiveCondenser`].
pub struct AdaptiveCondenserOptions {
    /// Safety margin multiplier for post-condense verification (e.g. 1.2 = 20% headroom).
    pub safety_margin: f64,
    /// Base fraction of messages to include in each summarization chunk (0.0..1.0).
    pub base_chunk_ratio: f64,
    /// Minimum chunk ratio floor (0.0..1.0).
    pub min_chunk_ratio: f64,
    /// A single message exceeding this fraction of the budget is considered oversized.
    pub oversize_threshold: f64,
    /// Hard limit on message count — triggers immediate trimming.
    pub max_messages: usize,
    /// Number of recent messages to always keep intact.
    pub keep_recent: usize,
}

impl Default for AdaptiveCondenserOptions {
    fn default() -> Self {
        Self {
            safety_margin: 1.2,
            base_chunk_ratio: 0.4,
            min_chunk_ratio: 0.15,
            oversize_threshold: 0.5,
            max_messages: 100,
            keep_recent: 10,
        }
    }
}

/// An adaptive condenser that uses an LLM to summarize older messages
/// when the conversation approaches the context window limit.
///
/// Implements a multi-step decision flow:
/// 1. Message count hard limit
/// 2. Token budget check (skip if within budget)
/// 3. Adaptive chunk ratio computation
/// 4. Oversized message detection
/// 5. Chunked LLM summarization with graceful fallback
/// 6. Post-condense verification with safety margin
/// 7. Final hard-truncation fallback
pub struct AdaptiveCondenser {
    model: Arc<dyn ChatModel>,
    options: AdaptiveCondenserOptions,
}

impl AdaptiveCondenser {
    pub fn new(model: Arc<dyn ChatModel>, options: AdaptiveCondenserOptions) -> Self {
        Self { model, options }
    }

    /// Summarize a chunk of messages by calling the LLM.
    async fn summarize_chunk(
        &self,
        chunk: &[Message],
        chunk_index: usize,
    ) -> Result<String, SynapticError> {
        let chunk_text = chunk
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let role = if m.is_system() {
                    "system"
                } else if m.is_human() {
                    "human"
                } else {
                    "assistant"
                };
                format!("[{}] {}: {}", i, role, m.content())
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize the following conversation chunk concisely, preserving key information.\n\
             IMPORTANT: You MUST preserve all identifiers exactly as they appear — \
             including IDs, hashes, commit SHAs, file paths, URLs, version numbers, UUIDs, \
             and any other specific references.\n\n{}",
            chunk_text
        );

        let input_tokens = estimate_text(&prompt);

        let req = ChatRequest::new(vec![
            Message::system("You are a conversation summarizer."),
            Message::human(prompt),
        ]);

        let response = self.model.chat(req).await?;
        let summary = response.message.content().to_string();
        let output_tokens = estimate_text(&summary);

        debug!(
            chunk_index,
            chunk_messages = chunk.len(),
            input_tokens,
            output_tokens,
            "chunk summarization completed"
        );

        Ok(summary)
    }

    /// Truncate a message's content, keeping first and last N chars.
    fn truncate_content(content: &str, keep_chars: usize) -> String {
        if content.len() <= keep_chars * 2 {
            return content.to_string();
        }
        let first = &content[..keep_chars];
        let last = &content[content.len() - keep_chars..];
        format!(
            "{}...[truncated {} chars]...{}",
            first,
            content.len() - keep_chars * 2,
            last
        )
    }
}

#[async_trait]
impl Condenser for AdaptiveCondenser {
    async fn condense(&self, ctx: CondenseContext) -> Result<CondenseResult, SynapticError> {
        let opts = &self.options;
        let budget = ctx.message_budget();
        let messages = ctx.messages;

        // ─── Step 1: Message count hard limit ───────────────────────────
        if messages.len() > opts.max_messages {
            let original = messages.len();
            let has_system = messages.first().is_some_and(|m| m.is_system());
            let keep_recent = opts.keep_recent.min(messages.len());

            let mut result = Vec::new();
            if has_system {
                result.push(messages[0].clone());
            }
            // Keep the most recent messages
            let start = messages.len().saturating_sub(keep_recent);
            result.extend(messages[start..].iter().cloned());

            let estimated_tokens = estimate_messages(&result);
            return Ok(CondenseResult {
                messages: result,
                estimated_tokens,
                action: CondenseAction::Degraded {
                    reason: format!(
                        "message count exceeded: {} > {}",
                        original, opts.max_messages
                    ),
                },
            });
        }

        // ─── Step 2: Token budget check ─────────────────────────────────
        let current_tokens = estimate_messages(&messages);
        if current_tokens < budget {
            debug!(
                current_tokens,
                budget, "within budget, skipping condensation"
            );
            return Ok(CondenseResult {
                messages,
                estimated_tokens: current_tokens,
                action: CondenseAction::Skip,
            });
        }

        // ─── Step 3: Compute adaptive chunk ratio ───────────────────────
        let avg_msg_tokens = if messages.is_empty() {
            0
        } else {
            current_tokens / messages.len()
        };
        let ratio = if budget > 0 && (avg_msg_tokens as f64 / budget as f64) > 0.1 {
            let fraction = avg_msg_tokens as f64 / budget as f64;
            (opts.base_chunk_ratio - fraction).max(opts.min_chunk_ratio)
        } else {
            opts.base_chunk_ratio
        };
        debug!(
            avg_msg_tokens,
            budget, ratio, "computed adaptive chunk ratio"
        );

        // ─── Step 4: Detect oversized messages ──────────────────────────
        let oversize_limit = (budget as f64 * opts.oversize_threshold) as usize;
        let mut oversized_indices = Vec::new();
        for (i, msg) in messages.iter().enumerate() {
            let msg_tokens = estimate_message_tokens(msg);
            if msg_tokens > oversize_limit {
                warn!(
                    index = i,
                    estimated_tokens = msg_tokens,
                    budget,
                    "oversized message detected"
                );
                oversized_indices.push(i);
            }
        }

        // ─── Step 5: Split and summarize ────────────────────────────────
        let has_system = messages.first().is_some_and(|m| m.is_system());
        let keep_recent = opts.keep_recent.min(messages.len());

        // Determine split point: [system?] + [old to summarize] + [keep_recent]
        let old_start = if has_system { 1 } else { 0 };
        let old_end = messages.len().saturating_sub(keep_recent);

        if old_end <= old_start {
            // Nothing to summarize — all messages are in the "keep" window
            let estimated_tokens = estimate_messages(&messages);
            return Ok(CondenseResult {
                messages,
                estimated_tokens,
                action: CondenseAction::Skip,
            });
        }

        let old_messages: Vec<Message> = messages[old_start..old_end].to_vec();
        let old_count = old_messages.len();

        // Build chunks from old messages, excluding oversized ones
        // The oversized_indices are absolute indices; convert to old_messages-relative
        let oversized_in_old: Vec<usize> = oversized_indices
            .iter()
            .filter_map(|&i| {
                if i >= old_start && i < old_end {
                    Some(i - old_start)
                } else {
                    None
                }
            })
            .collect();

        // Collect non-oversized messages for summarization
        let summarizable: Vec<(usize, &Message)> = old_messages
            .iter()
            .enumerate()
            .filter(|(i, _)| !oversized_in_old.contains(i))
            .collect();

        // Chunk the summarizable messages
        let chunk_size = ((summarizable.len() as f64 * ratio).ceil() as usize).max(1);
        let mut summaries: Vec<Message> = Vec::new();

        for chunk_msgs in summarizable.chunks(chunk_size) {
            let chunk_vec: Vec<Message> = chunk_msgs.iter().map(|(_, m)| (*m).clone()).collect();
            let chunk_idx = summaries.len();

            match self.summarize_chunk(&chunk_vec, chunk_idx).await {
                Ok(summary) => {
                    summaries.push(Message::system(format!("[Summary]: {}", summary)));
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        chunk_index = chunk_idx,
                        chunk_messages = chunk_vec.len(),
                        "summarization LLM call failure"
                    );
                    summaries.push(Message::system(format!(
                        "[Summary unavailable: {} messages]",
                        chunk_vec.len()
                    )));
                }
            }
        }

        // Reassemble: [system?] + summaries + [keep_recent]
        let mut result = Vec::new();
        if has_system {
            result.push(messages[0].clone());
        }
        result.extend(summaries);
        // Add back oversized messages from old range (they were excluded from summarization)
        for &oi in &oversized_in_old {
            result.push(old_messages[oi].clone());
        }
        result.extend(messages[old_end..].iter().cloned());

        // ─── Step 6: Post-condense verification ─────────────────────────
        let result_tokens = estimate_messages(&result);
        let safety_budget = (budget as f64 / opts.safety_margin) as usize;

        if result_tokens > safety_budget {
            // Truncate oversized messages
            let mut evicted_count = 0;
            for msg in &mut result {
                let msg_tokens = estimate_message_tokens(msg);
                if msg_tokens > oversize_limit {
                    let truncated = Self::truncate_content(msg.content(), 500);
                    *msg = Message::system(format!("[Truncated message]: {}", truncated));
                    evicted_count += 1;
                }
            }

            let final_tokens = estimate_messages(&result);

            // ─── Step 7: Final fallback ─────────────────────────────
            if final_tokens > safety_budget {
                // Hard truncate oldest messages until within budget
                let has_sys = result.first().is_some_and(|m| m.is_system());
                while estimate_messages(&result) > safety_budget && result.len() > 1 {
                    let remove_idx = if has_sys && result.len() > 1 { 1 } else { 0 };
                    result.remove(remove_idx);
                }

                let estimated_tokens = estimate_messages(&result);
                return Ok(CondenseResult {
                    messages: result,
                    estimated_tokens,
                    action: CondenseAction::Degraded {
                        reason: "budget exceeded after eviction".to_string(),
                    },
                });
            }

            let estimated_tokens = estimate_messages(&result);
            return Ok(CondenseResult {
                messages: result,
                estimated_tokens,
                action: CondenseAction::Evicted {
                    count: evicted_count,
                },
            });
        }

        let estimated_tokens = estimate_messages(&result);
        let kept = result.len();
        Ok(CondenseResult {
            messages: result,
            estimated_tokens,
            action: CondenseAction::Summarized {
                removed: old_count,
                kept,
            },
        })
    }
}
