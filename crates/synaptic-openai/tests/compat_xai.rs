use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use synaptic_models::{FakeBackend, ProviderResponse};
use synaptic_openai::compat::xai::{chat_model, config, XaiModel, BASE_URL};

fn openai_chat_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    })
}

#[test]
fn test_model_enum_as_str() {
    assert_eq!(XaiModel::Grok2Latest.as_str(), "grok-2-latest");
    assert_eq!(XaiModel::Grok2Mini.as_str(), "grok-2-mini");
    assert_eq!(XaiModel::GrokBeta.as_str(), "grok-beta");
    assert_eq!(XaiModel::Custom("grok-3".into()).as_str(), "grok-3");
}

#[test]
fn test_model_display() {
    assert_eq!(format!("{}", XaiModel::Grok2Latest), "grok-2-latest");
}

#[test]
fn test_config_base_url() {
    let cfg = config("xai-key", XaiModel::Grok2Latest.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
}

#[test]
fn test_config_fields() {
    let cfg = config("key", XaiModel::Grok2Latest.to_string())
        .with_temperature(0.7)
        .with_max_tokens(4096);
    assert_eq!(cfg.temperature, Some(0.7));
    assert_eq!(cfg.max_tokens, Some(4096));
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Hello from Grok!"),
    });
    let model = chat_model("xai-test-key", XaiModel::Grok2Latest.to_string(), backend);
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Hello from Grok!");
}

#[tokio::test]
async fn test_rate_limit_error() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 429,
        body: serde_json::json!({"error": {"message": "rate limited"}}),
    });
    let model = chat_model("xai-test-key", XaiModel::Grok2Mini.to_string(), backend);
    let err = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap_err();
    assert!(matches!(err, SynapticError::RateLimit(_)));
}

#[tokio::test]
async fn test_model_variant() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Legacy Grok response"),
    });
    let model = chat_model("xai-test-key", XaiModel::GrokBeta.to_string(), backend);
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hello!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Legacy Grok response");
}
