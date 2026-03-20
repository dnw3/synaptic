//! Mistral AI — OpenAI-compatible provider (chat + embeddings).

use std::sync::Arc;

use crate::ProviderBackend;

use super::super::{OpenAiChatModel, OpenAiConfig, OpenAiEmbeddings, OpenAiEmbeddingsConfig};

pub const BASE_URL: &str = "https://api.mistral.ai/v1";
pub const API_KEY_ENV: &str = "MISTRAL_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MistralModel {
    MistralLargeLatest,
    MistralSmallLatest,
    OpenMistralNemo,
    CodestralLatest,
    Custom(String),
}

impl MistralModel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::MistralLargeLatest => "mistral-large-latest",
            Self::MistralSmallLatest => "mistral-small-latest",
            Self::OpenMistralNemo => "open-mistral-nemo",
            Self::CodestralLatest => "codestral-latest",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for MistralModel {
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

pub fn embeddings_config(api_key: impl Into<String>) -> OpenAiEmbeddingsConfig {
    OpenAiEmbeddingsConfig::new(api_key).with_base_url(BASE_URL)
}

pub fn embeddings(
    api_key: impl Into<String>,
    model: impl Into<String>,
    backend: Arc<dyn ProviderBackend>,
) -> OpenAiEmbeddings {
    OpenAiEmbeddings::new(
        OpenAiEmbeddingsConfig::new(api_key)
            .with_model(model)
            .with_base_url(BASE_URL),
        backend,
    )
}
