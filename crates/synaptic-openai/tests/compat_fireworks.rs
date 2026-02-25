use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use synaptic_models::{FakeBackend, ProviderResponse};
use synaptic_openai::compat::fireworks::{chat_model, config, FireworksModel, BASE_URL};

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
        FireworksModel::Llama3_1_70bInstruct.as_str(),
        "accounts/fireworks/models/llama-v3p1-70b-instruct"
    );
    assert_eq!(
        FireworksModel::Llama3_1_8bInstruct.as_str(),
        "accounts/fireworks/models/llama-v3p1-8b-instruct"
    );
    assert_eq!(
        FireworksModel::DeepSeekR1.as_str(),
        "accounts/fireworks/models/deepseek-r1"
    );
    assert_eq!(
        FireworksModel::Qwen2_5_72bInstruct.as_str(),
        "accounts/fireworks/models/qwen2p5-72b-instruct"
    );
    assert_eq!(
        FireworksModel::Custom("my/model".into()).as_str(),
        "my/model"
    );
}

#[test]
fn test_model_display() {
    assert_eq!(
        format!("{}", FireworksModel::DeepSeekR1),
        "accounts/fireworks/models/deepseek-r1"
    );
}

#[test]
fn test_config_base_url() {
    let cfg = config("fw-key", FireworksModel::Llama3_1_70bInstruct.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
}

#[test]
fn test_config_fields() {
    let cfg = config("key", FireworksModel::Llama3_1_70bInstruct.to_string())
        .with_temperature(0.8)
        .with_max_tokens(2048)
        .with_top_p(0.95);
    assert_eq!(cfg.temperature, Some(0.8));
    assert_eq!(cfg.max_tokens, Some(2048));
    assert_eq!(cfg.top_p, Some(0.95));
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Hello from Fireworks!"),
    });
    let model = chat_model(
        "fw-test-key",
        FireworksModel::Llama3_1_70bInstruct.to_string(),
        backend,
    );
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Hello from Fireworks!");
}

#[tokio::test]
async fn test_rate_limit_error() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 429,
        body: serde_json::json!({"error": {"message": "rate limited"}}),
    });
    let model = chat_model(
        "fw-test-key",
        FireworksModel::Llama3_1_8bInstruct.to_string(),
        backend,
    );
    let err = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap_err();
    assert!(matches!(err, SynapticError::RateLimit(_)));
}

#[tokio::test]
async fn test_system_message() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("I am a helpful assistant."),
    });
    let model = chat_model(
        "fw-test-key",
        FireworksModel::DeepSeekR1.to_string(),
        backend,
    );
    let response = model
        .chat(ChatRequest::new(vec![
            Message::system("You are a helpful assistant."),
            Message::human("Who are you?"),
        ]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "I am a helpful assistant.");
}
