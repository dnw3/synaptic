//! Sandbox isolation for deep agent execution.
//!
//! Provides pluggable sandbox backends (Docker, SSH, plugins) that wrap
//! the `Backend` trait with process-level isolation.

pub mod types;

pub use types::*;
