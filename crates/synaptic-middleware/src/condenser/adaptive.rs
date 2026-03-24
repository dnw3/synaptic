use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{ChatModel, SynapticError};

use super::{CondenseAction, CondenseContext, CondenseResult, Condenser};

/// Options for the [`AdaptiveCondenser`].
pub struct AdaptiveCondenserOptions {
    /// Number of recent messages to always keep intact.
    pub keep_recent: usize,
    /// Trigger condensation when message tokens exceed this fraction of the budget (0.0..1.0).
    pub trigger_ratio: f64,
}

impl Default for AdaptiveCondenserOptions {
    fn default() -> Self {
        Self {
            keep_recent: 4,
            trigger_ratio: 0.85,
        }
    }
}

/// An adaptive condenser that uses an LLM to summarize older messages
/// when the conversation approaches the context window limit.
///
/// **STUB**: Full implementation is in Task 3. Currently returns [`CondenseAction::Skip`].
pub struct AdaptiveCondenser {
    #[allow(dead_code)]
    model: Arc<dyn ChatModel>,
    #[allow(dead_code)]
    options: AdaptiveCondenserOptions,
}

impl AdaptiveCondenser {
    pub fn new(model: Arc<dyn ChatModel>, options: AdaptiveCondenserOptions) -> Self {
        Self { model, options }
    }
}

#[async_trait]
impl Condenser for AdaptiveCondenser {
    async fn condense(&self, ctx: CondenseContext) -> Result<CondenseResult, SynapticError> {
        // STUB: full implementation in Task 3
        let estimated_tokens = ctx.estimate_message_tokens();
        Ok(CondenseResult {
            messages: ctx.messages,
            estimated_tokens,
            action: CondenseAction::Skip,
        })
    }
}
