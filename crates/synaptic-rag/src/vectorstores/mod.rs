mod in_memory;
mod multi_vector;

pub use in_memory::{InMemoryVectorStore, VectorStoreRetriever};
pub use multi_vector::MultiVectorRetriever;

// Re-export core traits/types for backward compatibility
pub use synaptic_core::{Document, Embeddings, Retriever, VectorStore};

#[cfg(feature = "qdrant")]
pub mod qdrant;

#[cfg(feature = "pinecone")]
pub mod pinecone;

#[cfg(feature = "chroma")]
pub mod chroma;

#[cfg(feature = "weaviate")]
pub mod weaviate;

#[cfg(feature = "elasticsearch")]
pub mod elasticsearch;

#[cfg(feature = "opensearch")]
pub mod opensearch;

#[cfg(feature = "milvus")]
pub mod milvus;

#[cfg(feature = "lancedb")]
pub mod lancedb;
