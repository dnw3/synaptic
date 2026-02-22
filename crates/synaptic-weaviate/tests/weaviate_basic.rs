use synaptic_weaviate::{WeaviateConfig, WeaviateVectorStore};

#[test]
fn config_new_sets_fields() {
    let config = WeaviateConfig::new("http", "localhost:8080", "Documents");
    assert_eq!(config.scheme, "http");
    assert_eq!(config.host, "localhost:8080");
    assert_eq!(config.class_name, "Documents");
    assert!(config.api_key.is_none());
}

#[test]
fn config_base_url() {
    let config = WeaviateConfig::new("http", "localhost:8080", "Documents");
    assert_eq!(config.base_url(), "http://localhost:8080");
}

#[test]
fn config_https_base_url() {
    let config = WeaviateConfig::new("https", "my-cluster.weaviate.network", "Articles");
    assert_eq!(config.base_url(), "https://my-cluster.weaviate.network");
}

#[test]
fn config_with_api_key() {
    let config = WeaviateConfig::new("https", "cluster.weaviate.network", "MyClass")
        .with_api_key("wcs-secret-key");
    assert_eq!(config.api_key.as_deref(), Some("wcs-secret-key"));
}

#[test]
fn config_builder_chain() {
    let config = WeaviateConfig::new("https", "my-cluster.weaviate.network", "Docs")
        .with_api_key("secret-key");

    assert_eq!(config.scheme, "https");
    assert_eq!(config.host, "my-cluster.weaviate.network");
    assert_eq!(config.class_name, "Docs");
    assert_eq!(config.api_key.as_deref(), Some("secret-key"));
}

#[test]
fn store_new_creates_instance() {
    let config = WeaviateConfig::new("http", "localhost:8080", "Documents");
    let store = WeaviateVectorStore::new(config);
    assert_eq!(store.config().class_name, "Documents");
}

#[test]
fn store_config_accessor() {
    let config = WeaviateConfig::new("http", "localhost:8080", "MyClass");
    let store = WeaviateVectorStore::new(config);
    assert_eq!(store.config().base_url(), "http://localhost:8080");
    assert_eq!(store.config().class_name, "MyClass");
}

// ---------------------------------------------------------------------------
// Integration tests — require a running Weaviate instance.
// Run with: cargo test -p synaptic-weaviate -- --ignored
// ---------------------------------------------------------------------------

#[cfg(test)]
mod integration {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use synaptic_core::{Document, Embeddings, SynapticError, VectorStore};
    use synaptic_weaviate::{WeaviateConfig, WeaviateVectorStore};

    struct FakeEmbeddings {
        dim: usize,
    }

    impl FakeEmbeddings {
        fn new(dim: usize) -> Self {
            Self { dim }
        }

        fn embed_text(&self, text: &str) -> Vec<f32> {
            let mut vec = vec![0.0f32; self.dim];
            for (i, byte) in text.bytes().enumerate() {
                vec[i % self.dim] += byte as f32 / 255.0;
            }
            let mag: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if mag > 0.0 {
                vec.iter_mut().for_each(|x| *x /= mag);
            }
            vec
        }
    }

    #[async_trait]
    impl Embeddings for FakeEmbeddings {
        async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SynapticError> {
            Ok(texts.iter().map(|t| self.embed_text(t)).collect())
        }

        async fn embed_query(&self, text: &str) -> Result<Vec<f32>, SynapticError> {
            Ok(self.embed_text(text))
        }
    }

    async fn setup_store(class_name: &str) -> WeaviateVectorStore {
        let config = WeaviateConfig::new("http", "localhost:8080", class_name);
        let store = WeaviateVectorStore::new(config);
        store
            .initialize()
            .await
            .expect("failed to initialize schema");
        store
    }

    #[tokio::test]
    #[ignore = "requires running Weaviate instance at localhost:8080"]
    async fn add_and_search_documents() {
        let store = setup_store("TestSearch").await;
        let embeddings = FakeEmbeddings::new(128);

        let docs = vec![
            Document::new("doc-1", "The quick brown fox jumps over the lazy dog"),
            Document::new("doc-2", "A fast red car drives down the highway"),
            Document::new("doc-3", "The lazy dog sleeps in the sun"),
        ];

        let ids = store.add_documents(docs, &embeddings).await.unwrap();
        assert_eq!(ids.len(), 3);

        let results = store
            .similarity_search("lazy dog sleeping", 2, &embeddings)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    #[ignore = "requires running Weaviate instance at localhost:8080"]
    async fn search_with_scores() {
        let store = setup_store("TestScores").await;
        let embeddings = FakeEmbeddings::new(128);

        let docs = vec![
            Document::new("s-1", "Rust programming language"),
            Document::new("s-2", "Python programming language"),
        ];

        store.add_documents(docs, &embeddings).await.unwrap();

        let results = store
            .similarity_search_with_score("Rust language", 2, &embeddings)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].1 >= results[1].1);
    }

    #[tokio::test]
    #[ignore = "requires running Weaviate instance at localhost:8080"]
    async fn empty_add_returns_empty_ids() {
        let store = setup_store("TestEmpty").await;
        let embeddings = FakeEmbeddings::new(128);
        let ids = store.add_documents(vec![], &embeddings).await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running Weaviate instance at localhost:8080"]
    async fn metadata_is_preserved() {
        let store = setup_store("TestMetadata").await;
        let embeddings = FakeEmbeddings::new(128);

        let mut meta = HashMap::new();
        meta.insert(
            "source".to_string(),
            serde_json::Value::String("test".into()),
        );

        let docs = vec![Document::with_metadata("m-1", "metadata test", meta)];
        store.add_documents(docs, &embeddings).await.unwrap();

        let results = store
            .similarity_search("metadata test", 1, &embeddings)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].metadata.get("source"),
            Some(&serde_json::Value::String("test".into()))
        );
    }
}
