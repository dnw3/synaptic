//! Zhipu AI (智谱清言 / GLM) — OpenAI-compatible provider.

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
pub const API_KEY_ENV: &str = "ZHIPU_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZhipuModel {
    GLM4Plus,
    GLM4,
    GLM4Flash,
    Custom(String),
}

impl ZhipuModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::GLM4Plus => "glm-4-plus",
            Self::GLM4 => "glm-4",
            Self::GLM4Flash => "glm-4-flash",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for ZhipuModel {
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
