mod adaptive;
mod middleware;
mod noop;
mod resolver;

pub use adaptive::{AdaptiveCondenser, AdaptiveCondenserOptions};
pub use middleware::CondenserMiddleware;
pub use noop::NoOpCondenser;
pub use resolver::{ContextWindowResolver, DefaultContextWindowResolver};

use async_trait::async_trait;
use synaptic_core::token_estimation::{
    estimate_messages, estimate_text, estimate_tools, THINKING_OUTPUT_RESERVE,
};
use synaptic_core::{Message, SynapticError, ToolDefinition};

/// All the context a [`Condenser`] needs to decide how to compact messages.
pub struct CondenseContext {
    /// The conversation messages to potentially condense.
    pub messages: Vec<Message>,
    /// The system prompt (for token budget calculation).
    pub system_prompt: String,
    /// Tool definitions currently registered (for token budget calculation).
    pub tools: Vec<ToolDefinition>,
    /// The model's total context window in tokens.
    pub context_window: usize,
    /// Tokens reserved for the model's output.
    pub reserved_output_tokens: usize,
    /// Whether the model uses extended thinking (reserves extra output tokens).
    pub has_thinking: bool,
}

impl CondenseContext {
    /// Effective output reservation, adding [`THINKING_OUTPUT_RESERVE`] when thinking is enabled.
    pub fn effective_output_reserve(&self) -> usize {
        if self.has_thinking {
            self.reserved_output_tokens + THINKING_OUTPUT_RESERVE
        } else {
            self.reserved_output_tokens
        }
    }

    /// Estimate the total token count of all messages.
    pub fn estimate_message_tokens(&self) -> usize {
        estimate_messages(&self.messages)
    }

    /// The token budget available for messages after accounting for output reserve,
    /// system prompt, tool definitions, and fixed overhead.
    pub fn message_budget(&self) -> usize {
        let output_reserve = self.effective_output_reserve();
        let system_tokens = estimate_text(&self.system_prompt);
        let tools_tokens = estimate_tools(&self.tools);
        // Fixed overhead for framing / separators
        let overhead: usize = 64;

        self.context_window
            .saturating_sub(output_reserve)
            .saturating_sub(system_tokens)
            .saturating_sub(tools_tokens)
            .saturating_sub(overhead)
    }
}

/// The result of a condensation operation.
pub struct CondenseResult {
    /// The (possibly condensed) messages.
    pub messages: Vec<Message>,
    /// Estimated token count of the returned messages.
    pub estimated_tokens: usize,
    /// What action the condenser took.
    pub action: CondenseAction,
}

/// Describes what a condenser did.
#[derive(Debug, Clone)]
pub enum CondenseAction {
    /// Messages were already within budget; no changes made.
    Skip,
    /// Older messages were summarized.
    Summarized { removed: usize, kept: usize },
    /// Messages were evicted (dropped without summarization).
    Evicted { count: usize },
    /// The condenser degraded the context in some way.
    Degraded { reason: String },
}

/// Trait for condensing (compressing) a message history within a token budget.
#[async_trait]
pub trait Condenser: Send + Sync {
    /// Condense the given context, returning a potentially shorter message list.
    async fn condense(&self, ctx: CondenseContext) -> Result<CondenseResult, SynapticError>;
}
