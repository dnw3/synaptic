//! Convenience constructors for OpenAI-compatible providers.
//!
//! Each provider uses the same wire format as OpenAI but with a different
//! base URL. The submodules pre-configure [`OpenAiConfig`](crate::OpenAiConfig)
//! and [`OpenAiEmbeddingsConfig`](crate::OpenAiEmbeddingsConfig) with the correct endpoint.

pub mod cohere;
pub mod deepseek;
pub mod fireworks;
pub mod groq;
pub mod huggingface;
pub mod mistral;
pub mod openrouter;
pub mod perplexity;
pub mod together;
pub mod xai;
