//! DeepSeek — OpenAI-compatible provider (ultra-low-cost reasoning).

use std::sync::Arc;

use synaptic_models::ProviderBackend;

use crate::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.deepseek.com/v1";
pub const API_KEY_ENV: &str = "DEEPSEEK_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepSeekModel {
    DeepSeekChat,
    DeepSeekReasoner,
    DeepSeekCoderV2,
    Custom(String),
}

impl DeepSeekModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DeepSeekChat => "deepseek-chat",
            Self::DeepSeekReasoner => "deepseek-reasoner",
            Self::DeepSeekCoderV2 => "deepseek-coder-v2",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for DeepSeekModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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
