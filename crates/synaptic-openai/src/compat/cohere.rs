//! Cohere — OpenAI-compatible provider (chat + embeddings).

use std::sync::Arc;

use synaptic_models::ProviderBackend;

use crate::{OpenAiChatModel, OpenAiConfig, OpenAiEmbeddings, OpenAiEmbeddingsConfig};

pub const BASE_URL: &str = "https://api.cohere.ai/compatibility/v1";
pub const API_KEY_ENV: &str = "COHERE_API_KEY";

pub fn config(api_key: impl Into<String>, model: impl Into<String>) -> OpenAiConfig {
    OpenAiConfig::new(api_key, model).with_base_url(BASE_URL)
}

pub fn chat_model(
    api_key: impl Into<String>,
    model: impl Into<String>,
    backend: Arc<dyn ProviderBackend>,
) -> OpenAiChatModel {
    OpenAiChatModel::new(config(api_key, model), backend)
}

pub fn embeddings_config(api_key: impl Into<String>) -> OpenAiEmbeddingsConfig {
    OpenAiEmbeddingsConfig::new(api_key).with_base_url(BASE_URL)
}

pub fn embeddings(
    api_key: impl Into<String>,
    backend: Arc<dyn ProviderBackend>,
) -> OpenAiEmbeddings {
    OpenAiEmbeddings::new(embeddings_config(api_key), backend)
}
