//! MongoDB Atlas Vector Search integration for Synaptic.
//!
//! This module provides:
//! - [`MongoVectorStore`]: An implementation of the [`VectorStore`](synaptic_core::VectorStore)
//!   trait backed by [MongoDB Atlas Vector Search](https://www.mongodb.com/docs/atlas/atlas-vector-search/).
//! - [`MongoCheckpointer`]: An implementation of the [`Checkpointer`](synaptic_graph::Checkpointer)
//!   trait for persisting graph state in MongoDB (requires `mongodb-checkpointer` feature).
//!
//! # Example
//!
//! ```rust,no_run
//! use synaptic_store::mongodb::{MongoVectorStore, MongoVectorConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = MongoVectorConfig::new("my_database", "my_collection");
//! let store = MongoVectorStore::from_uri("mongodb+srv://...", config).await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "mongodb-checkpointer")]
pub mod checkpointer;
mod vector_store;

#[cfg(feature = "mongodb-checkpointer")]
pub use checkpointer::MongoCheckpointer;
pub use vector_store::{MongoVectorConfig, MongoVectorStore};

// Re-export core traits for convenience.
pub use synaptic_core::{Document, Embeddings, VectorStore};
