use async_trait::async_trait;
use std::sync::Arc;
use synaptic_core::{Message, SynapticError};
use synaptic_deep::backend::{Backend, StateBackend};
use synaptic_deep::middleware::memory::DeepMemoryMiddleware;
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};

/// A mock ModelCaller that captures the request for inspection.
struct CapturingCaller;

#[async_trait]
impl ModelCaller for CapturingCaller {
    async fn call(&self, request: ModelRequest) -> Result<ModelResponse, SynapticError> {
        let content = request.system_prompt.unwrap_or_default();
        Ok(ModelResponse {
            message: Message::ai(content),
            usage: None,
        })
    }
}

fn empty_request() -> ModelRequest {
    ModelRequest {
        messages: vec![Message::human("hello")],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    }
}

#[tokio::test]
async fn loads_memory_file() {
    let backend = Arc::new(StateBackend::new());
    backend
        .write_file("AGENTS.md", "# Memory\n- Always use Rust.")
        .await
        .unwrap();

    let mw = DeepMemoryMiddleware::new(backend, "AGENTS.md".to_string());
    let caller = CapturingCaller;
    let request = empty_request();
    let response = mw.wrap_model_call(request, &caller).await.unwrap();

    let prompt = response.message.content().to_string();
    assert!(prompt.contains("<agent_memory>"));
    assert!(prompt.contains("Always use Rust"));
    assert!(prompt.contains("</agent_memory>"));
}

#[tokio::test]
async fn missing_memory_file_no_error() {
    let backend = Arc::new(StateBackend::new());
    let mw = DeepMemoryMiddleware::new(backend, "AGENTS.md".to_string());
    let caller = CapturingCaller;
    let request = empty_request();
    let response = mw.wrap_model_call(request, &caller).await.unwrap();
    assert!(response.message.content().is_empty());
}

#[tokio::test]
async fn appends_to_existing_prompt() {
    let backend = Arc::new(StateBackend::new());
    backend
        .write_file("mem.md", "Remember this.")
        .await
        .unwrap();

    let mw = DeepMemoryMiddleware::new(backend, "mem.md".to_string());
    let caller = CapturingCaller;
    let mut request = empty_request();
    request.system_prompt = Some("You are helpful.".to_string());
    let response = mw.wrap_model_call(request, &caller).await.unwrap();

    let prompt = response.message.content().to_string();
    assert!(prompt.starts_with("You are helpful."));
    assert!(prompt.contains("Remember this."));
}

#[tokio::test]
async fn empty_memory_file_no_injection() {
    let backend = Arc::new(StateBackend::new());
    backend.write_file("AGENTS.md", "").await.unwrap();

    let mw = DeepMemoryMiddleware::new(backend, "AGENTS.md".to_string());
    let caller = CapturingCaller;
    let request = empty_request();
    let response = mw.wrap_model_call(request, &caller).await.unwrap();
    assert!(response.message.content().is_empty());
}
