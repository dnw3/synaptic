//! Cohere Embeddings implementation using the native Cohere v2 API.
//!
//! Unlike the OpenAI-compatible endpoint, this implementation supports Cohere's
//! `input_type` parameter, which is required for optimal retrieval performance:
//! use `search_document` when embedding documents and `search_query` when embedding queries.

use async_trait::async_trait;
use serde_json::json;
use synaptic_core::{Embeddings, SynapticError};

/// Input type for Cohere embeddings.
///
/// Using the correct input type is important for retrieval quality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CohereInputType {
    /// For embedding documents to be stored in a vector database.
    SearchDocument,
    /// For embedding queries used to search the vector database.
    SearchQuery,
    /// For classification tasks.
    Classification,
    /// For clustering tasks.
    Clustering,
}

impl CohereInputType {
    pub fn as_str(&self) -> &str {
        match self {
            CohereInputType::SearchDocument => "search_document",
            CohereInputType::SearchQuery => "search_query",
            CohereInputType::Classification => "classification",
            CohereInputType::Clustering => "clustering",
        }
    }
}

/// Configuration for [`CohereEmbeddings`].
#[derive(Debug, Clone)]
pub struct CohereEmbeddingsConfig {
    pub api_key: String,
    /// Model name (default: `"embed-english-v3.0"`).
    pub model: String,
    /// Input type for document embedding (default: `SearchDocument`).
    pub input_type: CohereInputType,
    /// Query input type (default: `SearchQuery`).
    pub query_input_type: CohereInputType,
    /// Base URL (default: `"https://api.cohere.ai/v2"`).
    pub base_url: String,
}

impl CohereEmbeddingsConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "embed-english-v3.0".to_string(),
            input_type: CohereInputType::SearchDocument,
            query_input_type: CohereInputType::SearchQuery,
            base_url: "https://api.cohere.ai/v2".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_input_type(mut self, input_type: CohereInputType) -> Self {
        self.input_type = input_type;
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

/// Embeddings backed by the Cohere Embed API.
///
/// Supports all Cohere embedding models including `embed-english-v3.0` (1024-dim)
/// and `embed-multilingual-v3.0` (1024-dim).
pub struct CohereEmbeddings {
    config: CohereEmbeddingsConfig,
    client: reqwest::Client,
}

impl CohereEmbeddings {
    pub fn new(config: CohereEmbeddingsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(config: CohereEmbeddingsConfig, client: reqwest::Client) -> Self {
        Self { config, client }
    }

    async fn embed_with_type(
        &self,
        texts: &[&str],
        input_type: &CohereInputType,
    ) -> Result<Vec<Vec<f32>>, SynapticError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let body = json!({
            "model": self.config.model,
            "texts": texts,
            "input_type": input_type.as_str(),
            "embedding_types": ["float"],
        });

        let response = self
            .client
            .post(format!("{}/embed", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Embedding(format!("Cohere embed request: {e}")))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(SynapticError::Embedding(format!(
                "Cohere embed API error ({status}): {text}"
            )));
        }

        let resp_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SynapticError::Embedding(format!("Cohere embed parse: {e}")))?;

        let float_embeddings = resp_body["embeddings"]["float"]
            .as_array()
            .ok_or_else(|| SynapticError::Embedding("missing embeddings.float".to_string()))?;

        let mut result = Vec::with_capacity(float_embeddings.len());
        for embedding in float_embeddings {
            let vec = embedding
                .as_array()
                .ok_or_else(|| SynapticError::Embedding("embedding is not array".to_string()))?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            result.push(vec);
        }

        Ok(result)
    }
}

#[async_trait]
impl Embeddings for CohereEmbeddings {
    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SynapticError> {
        self.embed_with_type(texts, &self.config.input_type).await
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, SynapticError> {
        let mut results = self
            .embed_with_type(&[text], &self.config.query_input_type)
            .await?;
        results
            .pop()
            .ok_or_else(|| SynapticError::Embedding("empty response".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = CohereEmbeddingsConfig::new("test-key");
        assert_eq!(config.model, "embed-english-v3.0");
        assert_eq!(config.input_type, CohereInputType::SearchDocument);
        assert_eq!(config.query_input_type, CohereInputType::SearchQuery);
    }

    #[test]
    fn config_builder() {
        let config = CohereEmbeddingsConfig::new("key")
            .with_model("embed-multilingual-v3.0")
            .with_input_type(CohereInputType::Clustering);
        assert_eq!(config.model, "embed-multilingual-v3.0");
        assert_eq!(config.input_type, CohereInputType::Clustering);
    }
}
