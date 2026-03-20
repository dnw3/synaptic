//! Baichuan (百川) — OpenAI-compatible provider.

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.baichuan-ai.com/v1";
pub const API_KEY_ENV: &str = "BAICHUAN_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaichuanModel {
    Baichuan4,
    Baichuan3Turbo,
    Custom(String),
}

impl BaichuanModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Baichuan4 => "Baichuan4",
            Self::Baichuan3Turbo => "Baichuan3-Turbo",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for BaichuanModel {
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
