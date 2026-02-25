//! Perplexity AI — OpenAI-compatible provider (online search-augmented).

use std::sync::Arc;

use synaptic_models::ProviderBackend;

use crate::{OpenAiChatModel, OpenAiConfig};

pub const BASE_URL: &str = "https://api.perplexity.ai";
pub const API_KEY_ENV: &str = "PERPLEXITY_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerplexityModel {
    SonarLarge,
    SonarSmall,
    SonarHuge,
    SonarReasoningPro,
    Custom(String),
}

impl PerplexityModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SonarLarge => "sonar-large-online",
            Self::SonarSmall => "sonar-small-online",
            Self::SonarHuge => "sonar-huge-online",
            Self::SonarReasoningPro => "sonar-reasoning-pro",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for PerplexityModel {
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
