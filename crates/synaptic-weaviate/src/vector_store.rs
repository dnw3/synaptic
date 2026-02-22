use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{Document, Embeddings, SynapticError, VectorStore};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// WeaviateConfig
// ---------------------------------------------------------------------------

/// Configuration for connecting to a Weaviate instance.
#[derive(Debug, Clone)]
pub struct WeaviateConfig {
    /// HTTP scheme: `http` or `https`.
    pub scheme: String,
    /// Host and port, e.g. `localhost:8080` or `my-cluster.weaviate.network`.
    pub host: String,
    /// Weaviate class (collection) name. Must start with an uppercase letter.
    pub class_name: String,
    /// Optional API key for authentication (Weaviate Cloud Services).
    pub api_key: Option<String>,
}

impl WeaviateConfig {
    pub fn new(
        scheme: impl Into<String>,
        host: impl Into<String>,
        class_name: impl Into<String>,
    ) -> Self {
        Self {
            scheme: scheme.into(),
            host: host.into(),
            class_name: class_name.into(),
            api_key: None,
        }
    }

    /// Add an API key for authentication.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Build the base URL from scheme and host.
    pub fn base_url(&self) -> String {
        format!("{}://{}", self.scheme, self.host)
    }
}

// ---------------------------------------------------------------------------
// WeaviateVectorStore
// ---------------------------------------------------------------------------

/// Weaviate-backed vector store.
///
/// Implements [`VectorStore`] using the Weaviate v1 REST API:
/// - Batch add: `POST /v1/batch/objects`
/// - Similarity search: `POST /v1/graphql` with `nearVector`
/// - Delete: `DELETE /v1/objects/{class}/{id}`
///
/// Call [`WeaviateVectorStore::initialize`] once to create the class schema
/// before adding documents.
pub struct WeaviateVectorStore {
    config: WeaviateConfig,
    client: reqwest::Client,
}

impl WeaviateVectorStore {
    /// Create a new store with the given configuration.
    pub fn new(config: WeaviateConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(config: WeaviateConfig, client: reqwest::Client) -> Self {
        Self { config, client }
    }

    /// Return a reference to the configuration.
    pub fn config(&self) -> &WeaviateConfig {
        &self.config
    }

    /// Create the Weaviate class schema for this store (idempotent).
    ///
    /// Creates a class with `content` (text), `metadata` (text), and
    /// `docId` (text) properties. Uses the `cosine` distance metric via
    /// `"vectorizer": "none"` (caller supplies vectors).
    pub async fn initialize(&self) -> Result<(), SynapticError> {
        let url = format!("{}/v1/schema", self.config.base_url());

        let schema = json!({
            "class": self.config.class_name,
            "description": format!("Synaptic vector store: {}", self.config.class_name),
            "properties": [
                {
                    "name": "content",
                    "dataType": ["text"],
                    "description": "Document content"
                },
                {
                    "name": "docId",
                    "dataType": ["text"],
                    "description": "Original document ID"
                },
                {
                    "name": "metadata",
                    "dataType": ["text"],
                    "description": "JSON-serialized document metadata"
                }
            ],
            "vectorizer": "none"
        });

        let mut req = self.client.post(&url).json(&schema);
        if let Some(ref key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("Weaviate initialize: {e}")))?;

        let status = resp.status().as_u16();
        // 200 = created; 422 = class already exists — both are acceptable
        if status != 200 && status != 422 {
            let body = resp.text().await.unwrap_or_default();
            return Err(SynapticError::VectorStore(format!(
                "Weaviate schema error (HTTP {status}): {body}"
            )));
        }

        Ok(())
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref key) = self.config.api_key {
            req.header("Authorization", format!("Bearer {key}"))
        } else {
            req
        }
    }

    /// Execute a nearVector GraphQL query and return raw items.
    async fn near_vector_query(
        &self,
        vector: &[f32],
        k: usize,
        with_score: bool,
    ) -> Result<Vec<Value>, SynapticError> {
        let additional = if with_score {
            "_additional { id distance }"
        } else {
            "_additional { id }"
        };

        let graphql_query = format!(
            "{{ Get {{ {class}(limit: {k}, nearVector: {{ vector: {vector} }}) {{ content docId metadata {additional} }} }} }}",
            class = self.config.class_name,
            k = k,
            vector = serde_json::to_string(vector).unwrap_or_default(),
        );

        let url = format!("{}/v1/graphql", self.config.base_url());
        let req = self.apply_auth(
            self.client
                .post(&url)
                .json(&json!({ "query": graphql_query })),
        );

        let resp = req
            .send()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("Weaviate search: {e}")))?;

        let status = resp.status().as_u16();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("Weaviate search parse: {e}")))?;

        if status != 200 {
            return Err(SynapticError::VectorStore(format!(
                "Weaviate search error (HTTP {status}): {body}"
            )));
        }

        Ok(body["data"]["Get"][&self.config.class_name]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    fn item_to_document(item: &Value) -> Document {
        let content = item["content"].as_str().unwrap_or("").to_string();
        let id = item["docId"].as_str().unwrap_or("").to_string();
        let metadata: HashMap<String, Value> = item["metadata"]
            .as_str()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        Document {
            id,
            content,
            metadata,
        }
    }
}

#[async_trait]
impl VectorStore for WeaviateVectorStore {
    async fn add_documents(
        &self,
        documents: Vec<Document>,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<String>, SynapticError> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let texts: Vec<&str> = documents.iter().map(|d| d.content.as_str()).collect();
        let vectors = embeddings.embed_documents(&texts).await?;

        let mut objects = Vec::with_capacity(documents.len());
        let mut ids = Vec::with_capacity(documents.len());

        for (doc, vector) in documents.iter().zip(vectors.iter()) {
            let weaviate_id = Uuid::new_v4().to_string();
            ids.push(weaviate_id.clone());

            let metadata_str =
                serde_json::to_string(&doc.metadata).unwrap_or_else(|_| "{}".to_string());

            objects.push(json!({
                "class": self.config.class_name,
                "id": weaviate_id,
                "properties": {
                    "content": doc.content,
                    "docId": doc.id,
                    "metadata": metadata_str,
                },
                "vector": vector,
            }));
        }

        let url = format!("{}/v1/batch/objects", self.config.base_url());
        let body = json!({ "objects": objects });

        let req = self.apply_auth(self.client.post(&url).json(&body));
        let resp = req
            .send()
            .await
            .map_err(|e| SynapticError::VectorStore(format!("Weaviate batch add: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::VectorStore(format!(
                "Weaviate batch add error (HTTP {status}): {text}"
            )));
        }

        Ok(ids)
    }

    async fn similarity_search(
        &self,
        query: &str,
        k: usize,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<Document>, SynapticError> {
        let query_vector = embeddings.embed_query(query).await?;
        let items = self.near_vector_query(&query_vector, k, false).await?;
        Ok(items.iter().map(Self::item_to_document).collect())
    }

    async fn similarity_search_with_score(
        &self,
        query: &str,
        k: usize,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<(Document, f32)>, SynapticError> {
        let query_vector = embeddings.embed_query(query).await?;
        let items = self.near_vector_query(&query_vector, k, true).await?;
        Ok(items
            .iter()
            .map(|item| {
                let doc = Self::item_to_document(item);
                // Weaviate returns cosine distance (0=identical, 2=opposite)
                // Convert to similarity score: 1 - distance/2
                let distance = item["_additional"]["distance"].as_f64().unwrap_or(1.0) as f32;
                let score = 1.0 - distance / 2.0;
                (doc, score)
            })
            .collect())
    }

    async fn similarity_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
    ) -> Result<Vec<Document>, SynapticError> {
        let items = self.near_vector_query(embedding, k, false).await?;
        Ok(items.iter().map(Self::item_to_document).collect())
    }

    async fn delete(&self, ids: &[&str]) -> Result<(), SynapticError> {
        for id in ids {
            let url = format!(
                "{}/v1/objects/{}/{}",
                self.config.base_url(),
                self.config.class_name,
                id
            );
            let req = self.apply_auth(self.client.delete(&url));
            let resp = req
                .send()
                .await
                .map_err(|e| SynapticError::VectorStore(format!("Weaviate delete: {e}")))?;

            let status = resp.status().as_u16();
            // 204 = deleted; 404 = already gone (OK to ignore)
            if status != 204 && status != 404 {
                let text = resp.text().await.unwrap_or_default();
                return Err(SynapticError::VectorStore(format!(
                    "Weaviate delete error (HTTP {status}): {text}"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_base_url() {
        let cfg = WeaviateConfig::new("http", "localhost:8080", "Document");
        assert_eq!(cfg.base_url(), "http://localhost:8080");
    }

    #[test]
    fn config_with_api_key() {
        let cfg = WeaviateConfig::new("https", "cluster.weaviate.network", "MyClass")
            .with_api_key("wcs-secret-key");
        assert_eq!(cfg.api_key, Some("wcs-secret-key".to_string()));
    }

    #[test]
    fn config_class_name() {
        let cfg = WeaviateConfig::new("http", "localhost:8080", "SynapticDocs");
        assert_eq!(cfg.class_name, "SynapticDocs");
    }
}
