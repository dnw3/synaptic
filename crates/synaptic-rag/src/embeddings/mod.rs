mod cached;
mod fake;

pub use cached::CacheBackedEmbeddings;
pub use fake::FakeEmbeddings;

// Re-export the Embeddings trait from core (forward-declared there).
pub use synaptic_core::Embeddings;

#[cfg(feature = "huggingface")]
pub mod huggingface;

#[cfg(feature = "voyage")]
pub mod voyage;

#[cfg(feature = "jina")]
pub mod jina;

#[cfg(feature = "nomic")]
pub mod nomic;

#[cfg(feature = "flashrank")]
pub mod flashrank;
