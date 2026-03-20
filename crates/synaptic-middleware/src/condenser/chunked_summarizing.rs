use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};

use super::Condenser;

/// Summarizes messages in chunks — splits old messages into segments of `chunk_size`,
/// summarizes each independently, then concatenates all summaries.
///
/// Better than single-pass summarization for very long conversations because
/// each chunk fits comfortably in the model's context window.
pub struct ChunkedSummarizingCondenser {
    model: Arc<dyn ChatModel>,
    max_tokens: usize,
    keep_recent: usize,
    chunk_size: usize,
}

impl ChunkedSummarizingCondenser {
    /// Create a new chunked condenser.
    ///
    /// - `chunk_size`: number of messages per summarization chunk (default suggestion: 30).
    pub fn new(
        model: Arc<dyn ChatModel>,
        max_tokens: usize,
        keep_recent: usize,
        chunk_size: usize,
    ) -> Self {
        Self {
            model,
            max_tokens,
            keep_recent,
            chunk_size: chunk_size.max(5), // minimum 5 messages per chunk
        }
    }

    /// Summarize a chunk of messages via LLM.
    async fn summarize_chunk(&self, messages: &[Message]) -> Result<String, SynapticError> {
        let mut text = String::new();
        for msg in messages {
            text.push_str(&format!("{}: {}\n", msg.role(), msg.content()));
        }

        let prompt = format!(
            "Summarize the following conversation chunk concisely, preserving key information.\n\
             IMPORTANT: You MUST preserve all identifiers exactly as they appear — \
             including IDs, hashes, commit SHAs, file paths, URLs, version numbers, UUIDs, \
             and any other specific references.\n\n{}",
            text
        );

        let request = ChatRequest::new(vec![Message::human(prompt)]);
        let response = self.model.chat(request).await?;
        Ok(response.message.content().to_string())
    }
}

#[async_trait]
impl Condenser for ChunkedSummarizingCondenser {
    async fn condense(&self, messages: Vec<Message>) -> Result<Vec<Message>, SynapticError> {
        // If messages are within budget, return as-is
        let estimated_tokens: usize = messages.iter().map(|m| m.content().len() / 4 + 4).sum();
        if estimated_tokens <= self.max_tokens {
            return Ok(messages);
        }

        // Split into system (if any), old messages to summarize, and recent to keep
        let (system_msg, rest) = if !messages.is_empty() && messages[0].is_system() {
            (Some(messages[0].clone()), &messages[1..])
        } else {
            (None, messages.as_slice())
        };

        if rest.len() <= self.keep_recent {
            return Ok(messages);
        }

        let split = rest.len() - self.keep_recent;
        let to_summarize = &rest[..split];
        let to_keep = &rest[split..];

        // Split into chunks and summarize each
        let mut summaries = Vec::new();
        for chunk in to_summarize.chunks(self.chunk_size) {
            match self.summarize_chunk(chunk).await {
                Ok(summary) => summaries.push(summary),
                Err(e) => {
                    // On failure, use a fallback: just note the chunk was dropped
                    summaries.push(format!(
                        "[{} messages could not be summarized: {}]",
                        chunk.len(),
                        e
                    ));
                }
            }
        }

        let combined_summary = summaries.join("\n\n---\n\n");

        // Reassemble: system + combined summary + recent
        let mut result = Vec::new();
        if let Some(sys) = system_msg {
            result.push(sys);
        }
        result.push(Message::system(format!(
            "[Conversation Summary ({} chunks)]\n{}",
            summaries.len(),
            combined_summary
        )));
        result.extend_from_slice(to_keep);

        Ok(result)
    }
}
