//! Weaviate vector database integration for the Synaptic framework.
//!
//! [`WeaviateVectorStore`] implements the [`VectorStore`](synaptic_core::VectorStore) trait
//! using the [Weaviate](https://weaviate.io/) REST API v1. Weaviate is a cloud-native,
//! modular vector database with support for multi-tenancy and hybrid search.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use synaptic_weaviate::{WeaviateVectorStore, WeaviateConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = WeaviateConfig::new("http", "localhost:8080", "Documents");
//! let store = WeaviateVectorStore::new(config);
//! store.initialize().await?;
//! # Ok(())
//! # }
//! ```

mod vector_store;

pub use vector_store::{WeaviateConfig, WeaviateVectorStore};

// Re-export core traits for convenience.
pub use synaptic_core::{Document, Embeddings, VectorStore};
