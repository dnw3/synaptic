use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::SynapticError;
use tracing::{debug, info, warn};

use super::{CondenseAction, CondenseContext, Condenser, ContextWindowResolver};
use crate::{Interceptor, ModelRequest};

/// Middleware that applies a [`Condenser`] to messages before each model call.
///
/// Resolves the context window via a [`ContextWindowResolver`], builds a
/// [`CondenseContext`], and logs the condensation action.
pub struct CondenserMiddleware {
    condenser: Arc<dyn Condenser>,
    resolver: Arc<dyn ContextWindowResolver>,
    reserved_output_tokens: usize,
}

impl CondenserMiddleware {
    pub fn new(
        condenser: Arc<dyn Condenser>,
        resolver: Arc<dyn ContextWindowResolver>,
        reserved_output_tokens: usize,
    ) -> Self {
        Self {
            condenser,
            resolver,
            reserved_output_tokens,
        }
    }

    fn log_action(action: &CondenseAction) {
        match action {
            CondenseAction::Skip => {
                debug!("condenser: skip (messages within budget)");
            }
            CondenseAction::Summarized { removed, kept } => {
                info!(removed, kept, "condenser: summarized older messages");
            }
            CondenseAction::Evicted { count } => {
                warn!(count, "condenser: evicted messages without summarization");
            }
            CondenseAction::Degraded { reason } => {
                warn!(reason, "condenser: degraded context");
            }
        }
    }
}

#[async_trait]
impl Interceptor for CondenserMiddleware {
    fn name(&self) -> &str {
        "CondenserMiddleware"
    }

    async fn before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
        // ModelRequest does not carry model/provider, so we pass "unknown"
        // and rely on the resolver's fallback or pre-registered entries.
        let context_window = self.resolver.resolve("unknown", "unknown");

        let system_prompt = request.system_prompt.clone().unwrap_or_default();
        let has_thinking = request.thinking.is_some();

        // Take messages out of the request to avoid cloning
        let messages = std::mem::take(&mut request.messages);

        let ctx = CondenseContext {
            messages,
            system_prompt,
            tools: request.tools.clone(),
            context_window,
            reserved_output_tokens: self.reserved_output_tokens,
            has_thinking,
        };

        // Log token estimation breakdown before condensing
        let msg_tokens = ctx.estimate_message_tokens();
        let budget = ctx.message_budget();
        debug!(
            context_window,
            reserved_output = ctx.effective_output_reserve(),
            message_tokens = msg_tokens,
            message_budget = budget,
            num_messages = ctx.messages.len(),
            num_tools = ctx.tools.len(),
            "condenser: token estimation breakdown"
        );

        let result = self.condenser.condense(ctx).await?;

        Self::log_action(&result.action);

        // Put condensed messages back
        request.messages = result.messages;

        Ok(())
    }
}
