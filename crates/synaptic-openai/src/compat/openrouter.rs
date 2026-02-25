//! OpenRouter — OpenAI-compatible provider (multi-model router).

use std::sync::Arc;

use synaptic_models::ProviderBackend;

use crate::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const API_KEY_ENV: &str = "OPENROUTER_API_KEY";

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
