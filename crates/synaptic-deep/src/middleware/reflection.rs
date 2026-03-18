#![allow(deprecated)]

use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use synaptic_middleware::AgentMiddleware;

use crate::backend::Backend;

/// Configuration for the reflection middleware.
#[derive(Debug, Clone)]
pub struct ReflectionConfig {
    /// Minimum number of messages in conversation to trigger reflection (default: 6).
    pub min_messages: usize,
    /// Memory file to write reflections to (default: "MEMORY.md").
    pub memory_file: String,
    /// Maximum chars for the reflection output (default: 500).
    pub max_reflection_chars: usize,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            min_messages: 6,
            memory_file: "MEMORY.md".to_string(),
            max_reflection_chars: 500,
        }
    }
}

/// Middleware that performs post-session reflection to extract reusable patterns.
///
/// After each agent execution, if the conversation is long enough, it uses a
/// lightweight model to analyze the conversation and extract new insights,
/// appending them to a memory file in the backend.
///
/// Reflection errors are non-fatal — they are logged but never fail the agent.
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
pub struct ReflectionMiddleware {
    /// Lightweight model for reflection (e.g. haiku).
    reflection_model: Arc<dyn ChatModel>,
    backend: Arc<dyn Backend>,
    config: ReflectionConfig,
}

impl ReflectionMiddleware {
    pub fn new(reflection_model: Arc<dyn ChatModel>, backend: Arc<dyn Backend>) -> Self {
        Self {
            reflection_model,
            backend,
            config: ReflectionConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ReflectionConfig) -> Self {
        self.config = config;
        self
    }
}

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for ReflectionMiddleware {
    async fn after_agent(&self, messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        // Skip if conversation is too short
        if messages.len() < self.config.min_messages {
            return Ok(());
        }

        // Build a condensed view of the conversation (last N messages to save tokens)
        let recent: Vec<&Message> = messages
            .iter()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let conversation_summary = recent
            .iter()
            .map(|m| {
                format!(
                    "[{}]: {}",
                    m.role(),
                    truncate_for_reflection(m.content(), 300)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Load existing memory
        let existing_memory = self
            .backend
            .read_file(&self.config.memory_file, 0, 10000)
            .await
            .unwrap_or_default();

        // Build reflection prompt
        let reflection_prompt = format!(
            "You are a reflection engine. Analyze this conversation and extract ONLY genuinely \
             reusable insights.\n\n\
             ## Current Memory\n{}\n\n\
             ## Recent Conversation\n{}\n\n\
             ## Task\n\
             Extract 0-3 concise bullet points of NEW insights worth remembering. Categories:\n\
             - User preferences (communication style, tools, workflows)\n\
             - Recurring patterns (common tasks, frequent errors)\n\
             - Learned solutions (what worked, what didn't)\n\n\
             Rules:\n\
             - Do NOT repeat anything already in memory\n\
             - Do NOT include session-specific details (current task, temp files, etc.)\n\
             - If nothing is worth remembering, output exactly: NOTHING_NEW\n\
             - Be extremely selective — only genuinely reusable insights\n\
             - Output ONLY the bullet points, no preamble",
            if existing_memory.is_empty() {
                "(empty)"
            } else {
                &existing_memory
            },
            conversation_summary
        );

        // Call reflection model
        let request = ChatRequest::new(vec![Message::human(&reflection_prompt)]);

        let response = match self.reflection_model.chat(request).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("Reflection skipped: {}", e);
                return Ok(()); // Don't fail the agent for reflection errors
            }
        };

        let reflection = response.message.content().trim().to_string();

        // Skip if nothing new
        if reflection == "NOTHING_NEW" || reflection.is_empty() {
            tracing::debug!("Reflection: nothing new to remember");
            return Ok(());
        }

        // Truncate if too long
        let reflection = if reflection.len() > self.config.max_reflection_chars {
            reflection[..self.config.max_reflection_chars].to_string()
        } else {
            reflection
        };

        // Append to memory file
        let new_content = if existing_memory.is_empty() {
            format!("# Agent Memory\n\n{}\n", reflection)
        } else {
            format!("{}\n{}\n", existing_memory.trim_end(), reflection)
        };

        if let Err(e) = self
            .backend
            .write_file(&self.config.memory_file, &new_content)
            .await
        {
            tracing::debug!("Failed to write reflection: {}", e);
        } else {
            tracing::info!("Reflection: updated memory file");
        }

        Ok(())
    }
}

fn truncate_for_reflection(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
