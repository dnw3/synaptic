//! Alibaba Qwen (通义千问) — OpenAI-compatible provider.

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
pub const API_KEY_ENV: &str = "DASHSCOPE_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QwenModel {
    QwenMax,
    QwenPlus,
    QwenTurbo,
    QwenLong,
    Custom(String),
}

impl QwenModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::QwenMax => "qwen-max",
            Self::QwenPlus => "qwen-plus",
            Self::QwenTurbo => "qwen-turbo",
            Self::QwenLong => "qwen-long",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for QwenModel {
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
