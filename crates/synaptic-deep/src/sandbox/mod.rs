//! Sandbox isolation for deep agent execution.
//!
//! Provides pluggable sandbox backends (Docker, SSH, plugins) that wrap
//! the `Backend` trait with process-level isolation.

pub mod fs_bridge;
pub mod provider;
pub mod types;
pub mod validate;

#[cfg(feature = "sandbox-docker")]
pub mod docker;
#[cfg(feature = "sandbox-ssh")]
pub mod ssh;

pub use fs_bridge::{FsBridge, MountMapping};
pub use provider::*;
pub use types::*;
pub use validate::validate_sandbox_security;

#[cfg(feature = "sandbox-docker")]
pub use docker::{DockerProvider, DockerProviderConfig};
#[cfg(feature = "sandbox-ssh")]
pub use ssh::{SshProvider, SshProviderConfig, SshWorkspaceMode};
