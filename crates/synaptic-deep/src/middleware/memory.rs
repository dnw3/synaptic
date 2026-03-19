use async_trait::async_trait;
use std::sync::Arc;
use synaptic_core::SynapticError;
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};

use crate::backend::Backend;

/// Middleware that loads a memory file from the backend and injects it into the system prompt.
///
/// On each model call, reads the configured memory file (default `AGENTS.md`) and
/// appends its contents wrapped in `<agent_memory>` tags to the system prompt.
pub struct DeepMemoryMiddleware {
    backend: Arc<dyn Backend>,
    memory_file: String,
}

impl DeepMemoryMiddleware {
    pub fn new(backend: Arc<dyn Backend>, memory_file: String) -> Self {
        Self {
            backend,
            memory_file,
        }
    }
}

#[async_trait]
impl Interceptor for DeepMemoryMiddleware {
    async fn wrap_model_call(
        &self,
        mut request: ModelRequest,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        match self.backend.read_file(&self.memory_file, 0, 10000).await {
            Ok(content) if !content.is_empty() => {
                let section = format!("\n<agent_memory>\n{}\n</agent_memory>\n", content);
                if let Some(ref mut prompt) = request.system_prompt {
                    prompt.push_str(&section);
                } else {
                    request.system_prompt = Some(section);
                }
            }
            _ => {} // File not found or empty — silently skip
        }
        next.call(request).await
    }
}
