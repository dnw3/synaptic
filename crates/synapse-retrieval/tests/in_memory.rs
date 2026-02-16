use synapse_core::SynapseError;
use synapse_retrieval::{Document, InMemoryRetriever, Retriever};

#[tokio::test]
async fn retrieves_best_match() -> Result<(), SynapseError> {
    let retriever = InMemoryRetriever::new(vec![
        Document::new("1", "rust async tokio"),
        Document::new("2", "python notebooks"),
    ]);

    let docs = retriever.retrieve("tokio runtime", 1).await?;
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].id, "1");
    Ok(())
}
