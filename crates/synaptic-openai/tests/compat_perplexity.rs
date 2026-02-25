use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError};
use synaptic_models::{FakeBackend, ProviderResponse};
use synaptic_openai::compat::perplexity::{chat_model, config, PerplexityModel, BASE_URL};

fn openai_chat_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30},
        "citations": ["https://example.com/source1", "https://example.com/source2"]
    })
}

#[test]
fn test_model_enum_as_str() {
    assert_eq!(PerplexityModel::SonarLarge.as_str(), "sonar-large-online");
    assert_eq!(PerplexityModel::SonarSmall.as_str(), "sonar-small-online");
    assert_eq!(PerplexityModel::SonarHuge.as_str(), "sonar-huge-online");
    assert_eq!(
        PerplexityModel::SonarReasoningPro.as_str(),
        "sonar-reasoning-pro"
    );
    assert_eq!(
        PerplexityModel::Custom("sonar-v2".into()).as_str(),
        "sonar-v2"
    );
}

#[test]
fn test_model_display() {
    assert_eq!(
        format!("{}", PerplexityModel::SonarLarge),
        "sonar-large-online"
    );
    assert_eq!(
        format!("{}", PerplexityModel::SonarReasoningPro),
        "sonar-reasoning-pro"
    );
}

#[test]
fn test_config_base_url() {
    let cfg = config("pplx-key", PerplexityModel::SonarLarge.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
}

#[test]
fn test_config_fields() {
    let cfg = config("key", PerplexityModel::SonarLarge.to_string())
        .with_temperature(0.1)
        .with_max_tokens(2048);
    assert_eq!(cfg.temperature, Some(0.1));
    assert_eq!(cfg.max_tokens, Some(2048));
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("According to recent sources, Rust is memory-safe."),
    });
    let model = chat_model(
        "pplx-test-key",
        PerplexityModel::SonarLarge.to_string(),
        backend,
    );
    let response = model
        .chat(ChatRequest::new(vec![Message::human("What is Rust?")]))
        .await
        .unwrap();
    assert_eq!(
        response.message.content(),
        "According to recent sources, Rust is memory-safe."
    );
}

#[tokio::test]
async fn test_rate_limit_error() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 429,
        body: serde_json::json!({"error": {"message": "rate limited"}}),
    });
    let model = chat_model(
        "pplx-test-key",
        PerplexityModel::SonarSmall.to_string(),
        backend,
    );
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
        body: openai_chat_body("Let me reason step by step..."),
    });
    let model = chat_model(
        "pplx-test-key",
        PerplexityModel::SonarReasoningPro.to_string(),
        backend,
    );
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Solve: 2+2")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Let me reason step by step...");
}
