use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use synaptic_models::{FakeBackend, ProviderResponse};
use synaptic_openai::compat::groq::{chat_model, config, GroqModel, BASE_URL};

fn openai_chat_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    })
}

#[test]
fn test_model_enum_as_str() {
    assert_eq!(
        GroqModel::Llama3_3_70bVersatile.as_str(),
        "llama-3.3-70b-versatile"
    );
    assert_eq!(
        GroqModel::Llama3_1_8bInstant.as_str(),
        "llama-3.1-8b-instant"
    );
    assert_eq!(GroqModel::Mixtral8x7b32768.as_str(), "mixtral-8x7b-32768");
    assert_eq!(GroqModel::Custom("my-model".into()).as_str(), "my-model");
}

#[test]
fn test_config_base_url() {
    let cfg = config("gsk-key", GroqModel::Llama3_3_70bVersatile.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
}

#[test]
fn test_config_fields() {
    let cfg = config("key", GroqModel::Llama3_3_70bVersatile.to_string())
        .with_temperature(0.7)
        .with_max_tokens(1024)
        .with_seed(42);
    assert_eq!(cfg.temperature, Some(0.7));
    assert_eq!(cfg.max_tokens, Some(1024));
    assert_eq!(cfg.seed, Some(42));
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Hello from Groq!"),
    });
    let model = chat_model(
        "gsk-test",
        GroqModel::Llama3_3_70bVersatile.to_string(),
        backend,
    );
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Hello from Groq!");
}

#[tokio::test]
async fn test_rate_limit_error() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 429,
        body: serde_json::json!({"error": {"message": "rate limited"}}),
    });
    let model = chat_model(
        "gsk-test",
        GroqModel::Llama3_1_8bInstant.to_string(),
        backend,
    );
    let err = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap_err();
    assert!(matches!(err, SynapticError::RateLimit(_)));
}
