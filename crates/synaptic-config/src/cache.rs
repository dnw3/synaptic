//! LLM response caching: InMemory, Semantic, CachedChatModel.
//!
//! Requires the `cache` feature flag.

mod in_memory {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use async_trait::async_trait;
    use synaptic_core::{ChatResponse, SynapticError};
    use tokio::sync::RwLock;

    use synaptic_core::LlmCache;

    struct CacheEntry {
        response: ChatResponse,
        created_at: Instant,
    }

    /// In-memory LLM response cache with optional TTL expiration.
    pub struct InMemoryCache {
        store: RwLock<HashMap<String, CacheEntry>>,
        ttl: Option<Duration>,
    }

    impl InMemoryCache {
        /// Create a new cache with no TTL (entries never expire).
        pub fn new() -> Self {
            Self {
                store: RwLock::new(HashMap::new()),
                ttl: None,
            }
        }

        /// Create a new cache where entries expire after the given duration.
        pub fn with_ttl(duration: Duration) -> Self {
            Self {
                store: RwLock::new(HashMap::new()),
                ttl: Some(duration),
            }
        }
    }

    impl Default for InMemoryCache {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl LlmCache for InMemoryCache {
        async fn get(&self, key: &str) -> Result<Option<ChatResponse>, SynapticError> {
            let store = self.store.read().await;
            match store.get(key) {
                Some(entry) => {
                    if let Some(ttl) = self.ttl {
                        if entry.created_at.elapsed() > ttl {
                            return Ok(None);
                        }
                    }
                    Ok(Some(entry.response.clone()))
                }
                None => Ok(None),
            }
        }

        async fn put(&self, key: &str, response: &ChatResponse) -> Result<(), SynapticError> {
            let mut store = self.store.write().await;
            store.insert(
                key.to_string(),
                CacheEntry {
                    response: response.clone(),
                    created_at: Instant::now(),
                },
            );
            Ok(())
        }

        async fn clear(&self) -> Result<(), SynapticError> {
            let mut store = self.store.write().await;
            store.clear();
            Ok(())
        }
    }
}

mod semantic {
    use std::sync::Arc;

    use async_trait::async_trait;
    use synaptic_core::{ChatResponse, Embeddings, SynapticError};
    use tokio::sync::RwLock;

    use synaptic_core::LlmCache;

    struct SemanticEntry {
        embedding: Vec<f32>,
        response: ChatResponse,
    }

    /// Cache that uses embedding similarity to match semantically equivalent queries.
    ///
    /// When a cache lookup is performed, the key is embedded and compared against all
    /// stored entries using cosine similarity. If any entry exceeds the similarity
    /// threshold, its cached response is returned.
    pub struct SemanticCache {
        embeddings: Arc<dyn Embeddings>,
        entries: RwLock<Vec<SemanticEntry>>,
        similarity_threshold: f32,
    }

    impl SemanticCache {
        /// Create a new semantic cache with the given embeddings provider and similarity threshold.
        ///
        /// The threshold should be between 0.0 and 1.0. A typical value is 0.95, meaning
        /// only very similar queries will match.
        pub fn new(embeddings: Arc<dyn Embeddings>, similarity_threshold: f32) -> Self {
            Self {
                embeddings,
                entries: RwLock::new(Vec::new()),
                similarity_threshold,
            }
        }
    }

    #[async_trait]
    impl LlmCache for SemanticCache {
        async fn get(&self, key: &str) -> Result<Option<ChatResponse>, SynapticError> {
            let query_embedding = self.embeddings.embed_query(key).await.map_err(|e| {
                SynapticError::Cache(format!("embedding error during cache get: {e}"))
            })?;

            let entries = self.entries.read().await;
            let mut best_score = f32::NEG_INFINITY;
            let mut best_response = None;

            for entry in entries.iter() {
                let score = cosine_similarity(&query_embedding, &entry.embedding);
                if score >= self.similarity_threshold && score > best_score {
                    best_score = score;
                    best_response = Some(entry.response.clone());
                }
            }

            Ok(best_response)
        }

        async fn put(&self, key: &str, response: &ChatResponse) -> Result<(), SynapticError> {
            let embedding = self.embeddings.embed_query(key).await.map_err(|e| {
                SynapticError::Cache(format!("embedding error during cache put: {e}"))
            })?;

            let mut entries = self.entries.write().await;
            entries.push(SemanticEntry {
                embedding,
                response: response.clone(),
            });

            Ok(())
        }

        async fn clear(&self) -> Result<(), SynapticError> {
            let mut entries = self.entries.write().await;
            entries.clear();
            Ok(())
        }
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot / (mag_a * mag_b)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn cosine_similarity_identical_vectors() {
            let a = vec![1.0, 0.0, 0.0];
            let b = vec![1.0, 0.0, 0.0];
            let sim = cosine_similarity(&a, &b);
            assert!((sim - 1.0).abs() < 1e-6);
        }

        #[test]
        fn cosine_similarity_orthogonal_vectors() {
            let a = vec![1.0, 0.0];
            let b = vec![0.0, 1.0];
            let sim = cosine_similarity(&a, &b);
            assert!(sim.abs() < 1e-6);
        }

        #[test]
        fn cosine_similarity_empty_vectors() {
            let a: Vec<f32> = vec![];
            let b: Vec<f32> = vec![];
            assert_eq!(cosine_similarity(&a, &b), 0.0);
        }
    }
}

mod cached_model {
    use std::sync::Arc;

    use async_trait::async_trait;
    use synaptic_core::{ChatModel, ChatRequest, ChatResponse, ChatStream, SynapticError};

    use synaptic_core::LlmCache;

    /// A ChatModel wrapper that caches responses using an [`LlmCache`].
    ///
    /// Cache keys are generated by serializing the [`ChatRequest`] to JSON.
    /// Streaming (`stream_chat`) delegates directly to the inner model without caching.
    pub struct CachedChatModel {
        inner: Arc<dyn ChatModel>,
        cache: Arc<dyn LlmCache>,
    }

    impl CachedChatModel {
        /// Create a new cached model wrapping the given model and cache.
        pub fn new(inner: Arc<dyn ChatModel>, cache: Arc<dyn LlmCache>) -> Self {
            Self { inner, cache }
        }
    }

    #[async_trait]
    impl ChatModel for CachedChatModel {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapticError> {
            let key = serde_json::to_string(&request)
                .map_err(|e| SynapticError::Cache(format!("failed to serialize request: {e}")))?;

            // Check cache first
            if let Some(cached) = self.cache.get(&key).await? {
                return Ok(cached);
            }

            // Cache miss: call inner model
            let response = self.inner.chat(request).await?;

            // Store in cache
            self.cache.put(&key, &response).await?;

            Ok(response)
        }

        fn stream_chat(&self, request: ChatRequest) -> ChatStream<'_> {
            // Streaming bypasses the cache
            self.inner.stream_chat(request)
        }
    }
}

pub use cached_model::CachedChatModel;
pub use in_memory::InMemoryCache;
pub use semantic::SemanticCache;

// Re-export LlmCache trait from core for backward compatibility
pub use synaptic_core::LlmCache;
