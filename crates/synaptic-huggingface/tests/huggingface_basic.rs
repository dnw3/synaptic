use synaptic_huggingface::{HuggingFaceEmbeddings, HuggingFaceEmbeddingsConfig};

#[test]
fn config_defaults() {
    let config = HuggingFaceEmbeddingsConfig::new("BAAI/bge-small-en-v1.5");
    assert_eq!(config.model, "BAAI/bge-small-en-v1.5");
    assert!(config.api_key.is_none());
}

#[test]
fn config_with_api_key() {
    let config = HuggingFaceEmbeddingsConfig::new("model").with_api_key("hf_123");
    assert_eq!(config.api_key, Some("hf_123".to_string()));
}

#[test]
fn embeddings_new() {
    let config = HuggingFaceEmbeddingsConfig::new("BAAI/bge-small-en-v1.5");
    let _embeddings = HuggingFaceEmbeddings::new(config);
}
