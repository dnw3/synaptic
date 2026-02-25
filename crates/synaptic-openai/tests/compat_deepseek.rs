use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message};
use synaptic_models::{FakeBackend, ProviderResponse};
use synaptic_openai::compat::deepseek::{chat_model, config, DeepSeekModel, BASE_URL};

fn openai_chat_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    })
}

#[test]
fn test_model_enum_as_str() {
    assert_eq!(DeepSeekModel::DeepSeekChat.as_str(), "deepseek-chat");
    assert_eq!(
        DeepSeekModel::DeepSeekReasoner.as_str(),
        "deepseek-reasoner"
    );
    assert_eq!(DeepSeekModel::DeepSeekCoderV2.as_str(), "deepseek-coder-v2");
    assert_eq!(DeepSeekModel::Custom("x".into()).as_str(), "x");
}

#[test]
fn test_config_base_url() {
    let cfg = config("sk-key", DeepSeekModel::DeepSeekChat.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
    let cfg2 = config("sk-key", DeepSeekModel::DeepSeekChat.to_string()).with_max_tokens(2048);
    assert_eq!(cfg2.max_tokens, Some(2048));
}

#[tokio::test]
async fn test_basic_chat() {
    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: openai_chat_body("Hello!"),
    });
    let model = chat_model("sk-test", DeepSeekModel::DeepSeekChat.to_string(), backend);
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hi!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Hello!");
}
