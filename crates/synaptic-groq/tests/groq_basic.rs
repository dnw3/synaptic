use std::sync::Arc;
use synaptic_groq::{ChatModel, ChatRequest, GroqChatModel, GroqConfig, GroqModel, Message};
use synaptic_models::{FakeBackend, ProviderResponse};

fn openai_chat_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    })
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Hello from Groq!"),
    });
    let config = GroqConfig::new("gsk-test", GroqModel::Llama3_3_70bVersatile);
    let model = GroqChatModel::new(config, backend);
    let request = ChatRequest::new(vec![Message::human("Hi!")]);
    let response = model.chat(request).await.unwrap();
    assert_eq!(response.message.content(), "Hello from Groq!");
}

#[tokio::test]
async fn test_rate_limit_error() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 429,
        body: serde_json::json!({"error": {"message": "rate limited"}}),
    });
    let config = GroqConfig::new("gsk-test", GroqModel::Llama3_1_8bInstant);
    let model = GroqChatModel::new(config, backend);
    let request = ChatRequest::new(vec![Message::human("Hi!")]);
    let err = model.chat(request).await.unwrap_err();
    assert!(matches!(err, synaptic_groq::SynapticError::RateLimit(_)));
}
