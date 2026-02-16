use synapse::core::{Message, RunnableConfig, SynapseError};
use synapse::retrieval::Document;

#[test]
fn facade_reexports_core_types() {
    let msg = Message::human("hello");
    assert_eq!(msg.content(), "hello");

    let doc = Document::new("1", "test content");
    assert_eq!(doc.id, "1");

    let config = RunnableConfig::default();
    assert!(config.tags.is_empty());

    // Verify SynapseError is accessible through the facade.
    let err = SynapseError::Validation("test".into());
    assert!(matches!(err, SynapseError::Validation(_)));
}
