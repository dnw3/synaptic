use std::sync::Arc;
pub use synaptic_core::{ChatModel, ChatRequest, ChatResponse, ChatStream, Message, SynapticError};
use synaptic_models::ProviderBackend;
use synaptic_openai::{OpenAiChatModel, OpenAiConfig};
pub use synaptic_openai::{OpenAiEmbeddings, OpenAiEmbeddingsConfig};

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
            MistralModel::MistralLargeLatest => "mistral-large-latest",
            MistralModel::MistralSmallLatest => "mistral-small-latest",
            MistralModel::OpenMistralNemo => "open-mistral-nemo",
            MistralModel::CodestralLatest => "codestral-latest",
            MistralModel::Custom(s) => s.as_str(),
        }
    }
}
impl std::fmt::Display for MistralModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct MistralConfig {
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub seed: Option<u64>,
}
impl MistralConfig {
    pub fn new(api_key: impl Into<String>, model: MistralModel) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.to_string(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            seed: None,
        }
    }
    pub fn new_custom(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            seed: None,
        }
    }
    pub fn with_max_tokens(mut self, v: u32) -> Self {
        self.max_tokens = Some(v);
        self
    }
    pub fn with_temperature(mut self, v: f64) -> Self {
        self.temperature = Some(v);
        self
    }
    pub fn with_top_p(mut self, v: f64) -> Self {
        self.top_p = Some(v);
        self
    }
    pub fn with_stop(mut self, v: Vec<String>) -> Self {
        self.stop = Some(v);
        self
    }
    pub fn with_seed(mut self, v: u64) -> Self {
        self.seed = Some(v);
        self
    }
}
impl From<MistralConfig> for OpenAiConfig {
    fn from(c: MistralConfig) -> Self {
        let mut cfg =
            OpenAiConfig::new(c.api_key, c.model).with_base_url("https://api.mistral.ai/v1");
        if let Some(v) = c.max_tokens {
            cfg = cfg.with_max_tokens(v);
        }
        if let Some(v) = c.temperature {
            cfg = cfg.with_temperature(v);
        }
        if let Some(v) = c.top_p {
            cfg = cfg.with_top_p(v);
        }
        if let Some(v) = c.stop {
            cfg = cfg.with_stop(v);
        }
        if let Some(v) = c.seed {
            cfg = cfg.with_seed(v);
        }
        cfg
    }
}

pub struct MistralChatModel {
    inner: OpenAiChatModel,
}

impl MistralChatModel {
    pub fn new(config: MistralConfig, backend: Arc<dyn ProviderBackend>) -> Self {
        Self {
            inner: OpenAiChatModel::new(config.into(), backend),
        }
    }
}

#[async_trait::async_trait]
impl ChatModel for MistralChatModel {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapticError> {
        self.inner.chat(request).await
    }
    fn stream_chat(&self, request: ChatRequest) -> ChatStream<'_> {
        self.inner.stream_chat(request)
    }
}

pub fn mistral_embeddings(
    api_key: impl Into<String>,
    model: impl Into<String>,
    backend: Arc<dyn ProviderBackend>,
) -> OpenAiEmbeddings {
    let config = OpenAiEmbeddingsConfig::new(api_key)
        .with_model(model)
        .with_base_url("https://api.mistral.ai/v1");
    OpenAiEmbeddings::new(config, backend)
}
