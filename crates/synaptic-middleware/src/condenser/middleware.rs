use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::SynapticError;

use super::Condenser;
use crate::{Interceptor, ModelRequest};

/// Middleware that applies a condenser to messages before each model call.
pub struct CondenserMiddleware {
    condenser: Arc<dyn Condenser>,
}

impl CondenserMiddleware {
    pub fn new(condenser: Arc<dyn Condenser>) -> Self {
        Self { condenser }
    }
}

#[async_trait]
impl Interceptor for CondenserMiddleware {
    async fn before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
        request.messages = self.condenser.condense(request.messages.clone()).await?;
        Ok(())
    }
}
