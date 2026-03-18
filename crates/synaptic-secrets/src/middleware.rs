#![allow(deprecated)]

use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::SynapticError;
use synaptic_middleware::{AgentMiddleware, ModelRequest, ModelResponse};

use crate::SecretRegistry;

/// Middleware that masks secrets in AI outputs and injects them into prompts.
///
/// - `before_model`: injects secrets into the system prompt template
/// - `after_model`: masks any leaked secrets in the AI response
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
pub struct SecretMaskingMiddleware {
    registry: Arc<SecretRegistry>,
}

#[allow(deprecated)]
impl SecretMaskingMiddleware {
    pub fn new(registry: Arc<SecretRegistry>) -> Self {
        Self { registry }
    }
}

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for SecretMaskingMiddleware {
    async fn before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
        // Inject secrets into system prompt if present
        if let Some(ref prompt) = request.system_prompt {
            request.system_prompt = Some(self.registry.inject(prompt)?);
        }
        Ok(())
    }

    async fn after_model(
        &self,
        _request: &ModelRequest,
        response: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        // Mask any leaked secrets in the AI response
        let content = response.message.content().to_string();
        let masked = self.registry.mask_output(&content);
        if masked != content {
            response.message.set_content(masked);
        }
        Ok(())
    }
}
