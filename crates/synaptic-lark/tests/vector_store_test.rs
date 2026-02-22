use synaptic_lark::{LarkConfig, LarkVectorStore};

#[test]
fn constructor() {
    let store = LarkVectorStore::new(LarkConfig::new("cli", "secret"), "dataset_xxx");
    assert_eq!(store.dataset_id(), "dataset_xxx");
}

#[tokio::test]
async fn similarity_search_by_vector_unsupported() {
    use synaptic_core::VectorStore;
    let store = LarkVectorStore::new(LarkConfig::new("a", "b"), "ds");
    let result = store.similarity_search_by_vector(&[0.1f32, 0.2], 5).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not supported"));
}
