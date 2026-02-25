//! Groq — OpenAI-compatible provider (ultra-fast LPU inference).

use std::sync::Arc;

use synaptic_models::ProviderBackend;

use crate::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const API_KEY_ENV: &str = "GROQ_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroqModel {
    Llama3_3_70bVersatile,
    Llama3_1_8bInstant,
    Llama3_1_70bVersatile,
    Gemma2_9bIt,
    Mixtral8x7b32768,
    Custom(String),
}

impl GroqModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Llama3_3_70bVersatile => "llama-3.3-70b-versatile",
            Self::Llama3_1_8bInstant => "llama-3.1-8b-instant",
            Self::Llama3_1_70bVersatile => "llama-3.1-70b-versatile",
            Self::Gemma2_9bIt => "gemma2-9b-it",
            Self::Mixtral8x7b32768 => "mixtral-8x7b-32768",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for GroqModel {
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
