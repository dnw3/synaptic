//! Moonshot AI (Kimi) — OpenAI-compatible provider.

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.moonshot.cn/v1";
pub const API_KEY_ENV: &str = "MOONSHOT_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoonshotModel {
    MoonshotV1_8K,
    MoonshotV1_32K,
    MoonshotV1_128K,
    Custom(String),
}

impl MoonshotModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::MoonshotV1_8K => "moonshot-v1-8k",
            Self::MoonshotV1_32K => "moonshot-v1-32k",
            Self::MoonshotV1_128K => "moonshot-v1-128k",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for MoonshotModel {
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
