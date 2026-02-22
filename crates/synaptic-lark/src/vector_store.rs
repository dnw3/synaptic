use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use synaptic_core::{Document, Embeddings, SynapticError, VectorStore};

use crate::{auth::TokenCache, LarkConfig};

/// Vector store backed by Lark's Search API (dataset-based, server-side embedding).
///
/// `add_documents`: indexes raw text into a Lark Search dataset.
/// `similarity_search`: calls Lark's natural-language search — no external embeddings needed.
/// `similarity_search_by_vector`: NOT supported (Lark has no raw vector query endpoint).
pub struct LarkVectorStore {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
    dataset_id: String,
}

impl LarkVectorStore {
    /// Create a new `LarkVectorStore` targeting the given Lark Search dataset.
    pub fn new(config: LarkConfig, dataset_id: impl Into<String>) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
            dataset_id: dataset_id.into(),
        }
    }

    /// Return the dataset ID this store is configured to use.
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    fn check(body: &Value, ctx: &str) -> Result<(), SynapticError> {
        if body["code"].as_i64().unwrap_or(-1) != 0 {
            Err(SynapticError::VectorStore(format!(
                "LarkVectorStore ({ctx}): {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl VectorStore for LarkVectorStore {
    async fn add_documents(
        &self,
        docs: Vec<Document>,
        _embeddings: &dyn Embeddings,
    ) -> Result<Vec<String>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/search/v2/datasets/{}/documents/batch_create",
            self.base_url, self.dataset_id
        );
        let items: Vec<Value> = docs
            .iter()
            .map(|d| {
                json!({
                    "id": d.id,
                    "title": d.metadata.get("title").and_then(|v| v.as_str()).unwrap_or(&d.id),
                    "body": d.content,
                    "meta": d.metadata,
                })
            })
            .collect();

        let body = json!({ "documents": items });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("add_documents: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("add_documents parse: {e}")))?;
        Self::check(&rb, "add_documents")?;

        Ok(docs.iter().map(|d| d.id.clone()).collect())
    }

    async fn similarity_search(
        &self,
        query: &str,
        k: usize,
        _embeddings: &dyn Embeddings,
    ) -> Result<Vec<Document>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/search/v2/datasets/{}/search",
            self.base_url, self.dataset_id
        );
        let body = json!({ "query": query, "page_size": k });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("similarity_search: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("similarity_search parse: {e}")))?;
        Self::check(&rb, "similarity_search")?;

        let items = rb["data"]["items"].as_array().cloned().unwrap_or_default();
        Ok(items
            .iter()
            .map(|item| {
                let id = item["id"].as_str().unwrap_or("").to_string();
                let content = item["body"].as_str().unwrap_or("").to_string();
                let mut metadata: HashMap<String, Value> = HashMap::new();
                if let Some(m) = item["meta"].as_object() {
                    for (k, v) in m {
                        metadata.insert(k.clone(), v.clone());
                    }
                }
                if let Some(title) = item["title"].as_str() {
                    metadata.insert("title".to_string(), Value::String(title.to_string()));
                }
                Document {
                    id,
                    content,
                    metadata,
                }
            })
            .collect())
    }

    async fn similarity_search_with_score(
        &self,
        query: &str,
        k: usize,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<(Document, f32)>, SynapticError> {
        // Lark search doesn't return explicit scores; use 1.0 as a placeholder
        let docs = self.similarity_search(query, k, embeddings).await?;
        Ok(docs.into_iter().map(|d| (d, 1.0_f32)).collect())
    }

    async fn similarity_search_by_vector(
        &self,
        _embedding: &[f32],
        _k: usize,
    ) -> Result<Vec<Document>, SynapticError> {
        Err(SynapticError::VectorStore(
            "LarkVectorStore: similarity_search_by_vector not supported (Lark has no raw vector query)".to_string(),
        ))
    }

    async fn delete(&self, ids: &[&str]) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/search/v2/datasets/{}/documents/batch_delete",
            self.base_url, self.dataset_id
        );
        let body = json!({ "document_ids": ids });
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("delete: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("delete parse: {e}")))?;
        Self::check(&rb, "delete")
    }
}
