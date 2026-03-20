//! Together AI — OpenAI-compatible provider (open-source model marketplace).

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.together.xyz/v1";
pub const API_KEY_ENV: &str = "TOGETHER_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TogetherModel {
    Llama3_3_70bInstructTurbo,
    Llama3_1_8bInstructTurbo,
    Llama3_1_405bInstructTurbo,
    DeepSeekR1,
    Qwen2_5_72bInstructTurbo,
    Mixtral8x7bInstruct,
    Custom(String),
}

impl TogetherModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Llama3_3_70bInstructTurbo => "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            Self::Llama3_1_8bInstructTurbo => "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
            Self::Llama3_1_405bInstructTurbo => "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo",
            Self::DeepSeekR1 => "deepseek-ai/DeepSeek-R1",
            Self::Qwen2_5_72bInstructTurbo => "Qwen/Qwen2.5-72B-Instruct-Turbo",
            Self::Mixtral8x7bInstruct => "mistralai/Mixtral-8x7B-Instruct-v0.1",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for TogetherModel {
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
