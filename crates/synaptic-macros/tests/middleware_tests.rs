//! Integration tests for the middleware attribute macros.

use std::sync::Arc;

use synaptic_core::{Message, SynapticError};
use synaptic_macros::{after_model, before_model, dynamic_prompt, wrap_model_call};
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};

// ---------------------------------------------------------------------------
// #[before_model]
// ---------------------------------------------------------------------------

#[before_model]
async fn add_context(request: &mut ModelRequest) -> Result<(), SynapticError> {
    request.system_prompt = Some("Be helpful".into());
    Ok(())
}

#[tokio::test]
async fn test_before_model_interceptor() {
    let i: Arc<dyn Interceptor> = add_context();
    let mut req = ModelRequest {
        messages: vec![],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };
    i.before_model(&mut req).await.unwrap();
    assert_eq!(req.system_prompt.as_deref(), Some("Be helpful"));
}

// ---------------------------------------------------------------------------
// #[after_model]
// ---------------------------------------------------------------------------

#[after_model]
async fn log_response(
    _request: &ModelRequest,
    response: &mut ModelResponse,
) -> Result<(), SynapticError> {
    // Simulate modifying the response by replacing the message
    response.message = Message::ai(format!("logged: {}", response.message.content()));
    Ok(())
}

#[tokio::test]
async fn test_after_model_interceptor() {
    let i: Arc<dyn Interceptor> = log_response();
    let req = ModelRequest {
        messages: vec![],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };
    let mut resp = ModelResponse {
        message: Message::ai("original"),
        usage: None,
    };
    i.after_model(&req, &mut resp).await.unwrap();
    assert_eq!(resp.message.content(), "logged: original");
}

// ---------------------------------------------------------------------------
// #[dynamic_prompt]
// ---------------------------------------------------------------------------

#[dynamic_prompt]
fn custom_prompt(messages: &[Message]) -> String {
    format!("You have {} messages in context", messages.len())
}

#[tokio::test]
async fn test_dynamic_prompt_interceptor() {
    let i: Arc<dyn Interceptor> = custom_prompt();
    let mut req = ModelRequest {
        messages: vec![Message::human("hi"), Message::ai("hello")],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };
    i.before_model(&mut req).await.unwrap();
    assert_eq!(
        req.system_prompt.as_deref(),
        Some("You have 2 messages in context")
    );
}

// ---------------------------------------------------------------------------
// Verify interceptor struct names (factory returns Arc)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_interceptor_names() {
    let _add_context: Arc<dyn Interceptor> = add_context();
    let _log_response: Arc<dyn Interceptor> = log_response();
    let _custom_prompt: Arc<dyn Interceptor> = custom_prompt();
}

// ---------------------------------------------------------------------------
// #[wrap_model_call] — compilation test
// ---------------------------------------------------------------------------

#[wrap_model_call]
async fn passthrough_model(
    request: ModelRequest,
    next: &dyn ModelCaller,
) -> Result<ModelResponse, SynapticError> {
    next.call(request).await
}

#[tokio::test]
async fn test_wrap_model_call_interceptor() {
    let _i: Arc<dyn Interceptor> = passthrough_model();
}

// ===========================================================================
// #[field] support tests
// ===========================================================================

// ---------------------------------------------------------------------------
// #[before_model] with #[field]
// ---------------------------------------------------------------------------

#[before_model]
async fn inject_prompt(
    #[field] prompt: String,
    request: &mut ModelRequest,
) -> Result<(), SynapticError> {
    request.system_prompt = Some(prompt);
    Ok(())
}

#[tokio::test]
async fn test_before_model_with_field() {
    let i: Arc<dyn Interceptor> = inject_prompt("You are a pirate".to_string());
    let mut req = ModelRequest {
        messages: vec![],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };
    i.before_model(&mut req).await.unwrap();
    assert_eq!(req.system_prompt.as_deref(), Some("You are a pirate"));
}

// ---------------------------------------------------------------------------
// #[after_model] with #[field]
// ---------------------------------------------------------------------------

#[after_model]
async fn tag_response(
    #[field] tag: String,
    _request: &ModelRequest,
    response: &mut ModelResponse,
) -> Result<(), SynapticError> {
    response.message = Message::ai(format!("[{}] {}", tag, response.message.content()));
    Ok(())
}

#[tokio::test]
async fn test_after_model_with_field() {
    let i: Arc<dyn Interceptor> = tag_response("v2".to_string());
    let req = ModelRequest {
        messages: vec![],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };
    let mut resp = ModelResponse {
        message: Message::ai("hi"),
        usage: None,
    };
    i.after_model(&req, &mut resp).await.unwrap();
    assert_eq!(resp.message.content(), "[v2] hi");
}

// ---------------------------------------------------------------------------
// #[dynamic_prompt] with #[field]
// ---------------------------------------------------------------------------

#[dynamic_prompt]
fn branded_prompt(#[field] brand: String, messages: &[Message]) -> String {
    format!("[{}] You have {} messages", brand, messages.len())
}

#[tokio::test]
async fn test_dynamic_prompt_with_field() {
    let i: Arc<dyn Interceptor> = branded_prompt("Acme".to_string());
    let mut req = ModelRequest {
        messages: vec![Message::human("hi")],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };
    i.before_model(&mut req).await.unwrap();
    assert_eq!(
        req.system_prompt.as_deref(),
        Some("[Acme] You have 1 messages")
    );
}

// ---------------------------------------------------------------------------
// #[wrap_model_call] with #[field] — compilation test
// ---------------------------------------------------------------------------

#[wrap_model_call]
async fn retry_model(
    #[field] max_retries: usize,
    request: ModelRequest,
    next: &dyn ModelCaller,
) -> Result<ModelResponse, SynapticError> {
    let mut last_err = None;
    for attempt in 0..=max_retries {
        match next.call(request.clone()).await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                let _ = attempt; // suppress unused warning
            }
        }
    }
    Err(last_err.unwrap())
}

#[tokio::test]
async fn test_wrap_model_call_with_field() {
    let _i: Arc<dyn Interceptor> = retry_model(3);
}

// ---------------------------------------------------------------------------
// #[wrap_tool_call] with #[field] — compilation test
// ---------------------------------------------------------------------------

use synaptic_macros::wrap_tool_call;
use synaptic_middleware::ToolCaller;

#[wrap_tool_call]
async fn logged_tool_call(
    #[field] log_prefix: String,
    request: synaptic_middleware::ToolCallRequest,
    next: &dyn ToolCaller,
) -> Result<serde_json::Value, SynapticError> {
    let _ = format!("{}: calling {}", log_prefix, request.call.name);
    next.call(request).await
}

#[tokio::test]
async fn test_wrap_tool_call_with_field() {
    let _i: Arc<dyn Interceptor> = logged_tool_call("LOG".to_string());
}
