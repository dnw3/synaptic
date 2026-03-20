//! xAI Grok — OpenAI-compatible provider.

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.x.ai/v1";
pub const API_KEY_ENV: &str = "XAI_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XaiModel {
    Grok2Latest,
    Grok2Mini,
    GrokBeta,
    Custom(String),
}

impl XaiModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Grok2Latest => "grok-2-latest",
            Self::Grok2Mini => "grok-2-mini",
            Self::GrokBeta => "grok-beta",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for XaiModel {
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
