use std::sync::Arc;
use synaptic_mistral::{
    ChatModel, ChatRequest, Message, MistralChatModel, MistralConfig, MistralModel,
};
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
        body: openai_chat_body("Bonjour!"),
    });
    let config = MistralConfig::new("key", MistralModel::MistralLargeLatest);
    let model = MistralChatModel::new(config, backend);
    let request = ChatRequest::new(vec![Message::human("Hello!")]);
    let response = model.chat(request).await.unwrap();
    assert_eq!(response.message.content(), "Bonjour!");
}
