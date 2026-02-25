use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use synaptic_models::{FakeBackend, ProviderResponse};
use synaptic_openai::compat::together::{chat_model, config, TogetherModel, BASE_URL};

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
        TogetherModel::Llama3_3_70bInstructTurbo.as_str(),
        "meta-llama/Llama-3.3-70B-Instruct-Turbo"
    );
    assert_eq!(
        TogetherModel::Llama3_1_8bInstructTurbo.as_str(),
        "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo"
    );
    assert_eq!(
        TogetherModel::DeepSeekR1.as_str(),
        "deepseek-ai/DeepSeek-R1"
    );
    assert_eq!(
        TogetherModel::Qwen2_5_72bInstructTurbo.as_str(),
        "Qwen/Qwen2.5-72B-Instruct-Turbo"
    );
    assert_eq!(
        TogetherModel::Custom("my-model".into()).as_str(),
        "my-model"
    );
}

#[test]
fn test_model_display() {
    assert_eq!(
        format!("{}", TogetherModel::DeepSeekR1),
        "deepseek-ai/DeepSeek-R1"
    );
}

#[test]
fn test_config_base_url() {
    let cfg = config("key", TogetherModel::Llama3_3_70bInstructTurbo.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
}

#[test]
fn test_config_fields() {
    let cfg = config("key", TogetherModel::Llama3_3_70bInstructTurbo.to_string())
        .with_temperature(0.7)
        .with_max_tokens(1024)
        .with_top_p(0.9);
    assert_eq!(cfg.temperature, Some(0.7));
    assert_eq!(cfg.max_tokens, Some(1024));
    assert_eq!(cfg.top_p, Some(0.9));
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Hello from Together AI!"),
    });
    let model = chat_model(
        "test-key",
        TogetherModel::Llama3_3_70bInstructTurbo.to_string(),
        backend,
    );
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Hello from Together AI!");
}

#[tokio::test]
async fn test_rate_limit_error() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 429,
        body: serde_json::json!({"error": {"message": "rate limited"}}),
    });
    let model = chat_model(
        "test-key",
        TogetherModel::Llama3_1_8bInstructTurbo.to_string(),
        backend,
    );
    let err = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap_err();
    assert!(matches!(err, SynapticError::RateLimit(_)));
}

#[tokio::test]
async fn test_custom_model() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Custom model response"),
    });
    let model = chat_model("test-key", "custom/model-v1", backend);
    let response = model
        .chat(ChatRequest::new(vec![
            Message::system("You are helpful."),
            Message::human("Hello!"),
        ]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Custom model response");
}
