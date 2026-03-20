//! RAG (Retrieval-Augmented Generation) components for the Synaptic framework.
//!
//! This crate consolidates embeddings, vector stores, retrieval, loaders, splitters,
//! prompts, eval, and parsers into a single feature-gated crate.

#[cfg(feature = "embeddings")]
pub mod embeddings;

#[cfg(feature = "vectorstores")]
pub mod vectorstores;

#[cfg(feature = "retrieval")]
pub mod retrieval;

#[cfg(feature = "loaders")]
pub mod loaders;

#[cfg(feature = "splitters")]
pub mod splitters;

#[cfg(feature = "prompts")]
pub mod prompts;

#[cfg(feature = "eval")]
pub mod eval;

#[cfg(feature = "parsers")]
pub mod parsers;
