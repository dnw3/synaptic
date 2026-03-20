//! Third-party integrations for the Synaptic framework.
//!
//! This crate consolidates confluence, tavily, slack, voice, scheduler,
//! and langfuse into a single feature-gated crate.

#[cfg(feature = "confluence")]
pub mod confluence;

#[cfg(feature = "tavily")]
pub mod tavily;

#[cfg(feature = "slack")]
pub mod slack;

#[cfg(feature = "voice")]
pub mod voice;

#[cfg(feature = "scheduler")]
pub mod scheduler;

#[cfg(feature = "langfuse")]
pub mod langfuse;
