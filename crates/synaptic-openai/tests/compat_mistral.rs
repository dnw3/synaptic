use synaptic_openai::compat::mistral::{config, embeddings_config, MistralModel, BASE_URL};

#[test]
fn test_model_enum_as_str() {
    assert_eq!(
        MistralModel::MistralLargeLatest.as_str(),
        "mistral-large-latest"
    );
    assert_eq!(
        MistralModel::MistralSmallLatest.as_str(),
        "mistral-small-latest"
    );
    assert_eq!(MistralModel::OpenMistralNemo.as_str(), "open-mistral-nemo");
    assert_eq!(MistralModel::CodestralLatest.as_str(), "codestral-latest");
    assert_eq!(MistralModel::Custom("x".into()).as_str(), "x");
}

#[test]
fn test_config_base_url() {
    let cfg = config("key", MistralModel::MistralLargeLatest.to_string());
    assert_eq!(cfg.base_url, BASE_URL);
    assert_eq!(cfg.temperature, None);

    let cfg2 = config("key", MistralModel::MistralLargeLatest.to_string()).with_temperature(0.3);
    assert_eq!(cfg2.temperature, Some(0.3));
}

#[test]
fn test_embeddings_config_base_url() {
    let cfg = embeddings_config("key");
    assert_eq!(cfg.base_url, BASE_URL);
}

#[tokio::test]
async fn test_basic_chat() {
    use std::sync::Arc;
    use synaptic_core::{ChatModel, ChatRequest, Message};
    use synaptic_models::{FakeBackend, ProviderResponse};
    use synaptic_openai::compat::mistral::chat_model;

    let backend = Arc::new(FakeBackend::new());
    backend.push_response(ProviderResponse {
        status: 200,
        body: serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{"message": {"role": "assistant", "content": "Bonjour!"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }),
    });
    let model = chat_model("key", MistralModel::MistralLargeLatest.to_string(), backend);
    let response = model
        .chat(ChatRequest::new(vec![Message::human("Hello!")]))
        .await
        .unwrap();
    assert_eq!(response.message.content(), "Bonjour!");
}
