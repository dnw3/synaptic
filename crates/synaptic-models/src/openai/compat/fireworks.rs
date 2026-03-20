//! Fireworks AI — OpenAI-compatible provider (ultra-fast open model inference).

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.fireworks.ai/inference/v1";
pub const API_KEY_ENV: &str = "FIREWORKS_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FireworksModel {
    Llama3_1_70bInstruct,
    Llama3_1_8bInstruct,
    DeepSeekR1,
    Qwen2_5_72bInstruct,
    Custom(String),
}

impl FireworksModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Llama3_1_70bInstruct => "accounts/fireworks/models/llama-v3p1-70b-instruct",
            Self::Llama3_1_8bInstruct => "accounts/fireworks/models/llama-v3p1-8b-instruct",
            Self::DeepSeekR1 => "accounts/fireworks/models/deepseek-r1",
            Self::Qwen2_5_72bInstruct => "accounts/fireworks/models/qwen2p5-72b-instruct",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for FireworksModel {
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
