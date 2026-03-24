use async_trait::async_trait;
use synaptic_core::SynapticError;

use super::{CondenseAction, CondenseContext, CondenseResult, Condenser};

/// A no-op condenser that returns messages unchanged.
pub struct NoOpCondenser;

#[async_trait]
impl Condenser for NoOpCondenser {
    async fn condense(&self, ctx: CondenseContext) -> Result<CondenseResult, SynapticError> {
        let estimated_tokens = ctx.estimate_message_tokens();
        Ok(CondenseResult {
            messages: ctx.messages,
            estimated_tokens,
            action: CondenseAction::Skip,
        })
    }
}
