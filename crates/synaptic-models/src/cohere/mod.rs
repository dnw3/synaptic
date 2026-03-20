//! Cohere integration for Synaptic.
//!
//! This module provides [`CohereReranker`], a reranker that uses the
//! [Cohere Rerank API](https://docs.cohere.com/reference/rerank) to
//! reorder documents by relevance to a query.
//!
//! When the `cohere-retrieval` feature is enabled, `CohereReranker` also implements
//! the [`DocumentCompressor`](synaptic_retrieval::DocumentCompressor) trait,
//! making it usable with
//! [`ContextualCompressionRetriever`](synaptic_retrieval::ContextualCompressionRetriever).

mod embeddings;
mod reranker;

pub use embeddings::{CohereEmbeddings, CohereEmbeddingsConfig, CohereInputType};
pub use reranker::{CohereReranker, CohereRerankerConfig};

// Re-export core types for convenience.
pub use synaptic_core::Document;
