//! ByteDance Ark (火山方舟) — OpenAI-compatible provider.

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://ark.cn-beijing.volces.com/api/v3";
pub const API_KEY_ENV: &str = "ARK_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArkModel {
    DoubaoLite,
    DoubaoPlus,
    DoubaoMax,
    Custom(String),
}

impl ArkModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DoubaoLite => "doubao-1.5-lite-32k",
            Self::DoubaoPlus => "doubao-1.5-plus-32k",
            Self::DoubaoMax => "doubao-1.5-max-256k",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for ArkModel {
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
