//! Container sandbox for secure code execution.
//!
//! Provides a [`SandboxBackend`] trait and concrete implementations for running
//! untrusted code inside isolated containers with configurable resource limits
//! (memory, CPU, network, timeout).
//!
//! # Backends
//!
//! - **Docker** (feature `sandbox-docker`) — [`docker::DockerSandbox`]
//! - **Apple Container** (feature `sandbox-apple`, macOS only) — [`apple::AppleContainerSandbox`]
//!
//! Requires the `sandbox` feature flag.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

// ---------------------------------------------------------------------------
// Feature-gated backend modules
// ---------------------------------------------------------------------------

#[cfg(feature = "sandbox-docker")]
pub mod docker;

pub mod apple;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Result of a sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxResult {
    /// Standard output from the executed code.
    pub stdout: String,
    /// Standard error from the executed code.
    pub stderr: String,
    /// Process exit code (0 = success).
    pub exit_code: i32,
}

/// Resource limits applied to a sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in megabytes.
    pub memory_mb: u64,
    /// Number of CPUs (fractional allowed).
    pub cpu_count: f64,
    /// Execution timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to allow network access.
    pub network: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_count: 1.0,
            timeout_secs: 30,
            network: false,
        }
    }
}

// ---------------------------------------------------------------------------
// SandboxBackend trait
// ---------------------------------------------------------------------------

/// Trait for container sandbox backends.
///
/// Implementors provide isolated code execution with resource constraints.
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    /// Execute `code` written in `language` with the given resource limits.
    async fn execute(
        &self,
        language: &str,
        code: &str,
        limits: &ResourceLimits,
    ) -> Result<SandboxResult, SynapticError>;

    /// Check whether this backend is available on the current system.
    async fn is_available(&self) -> bool;
}

// ---------------------------------------------------------------------------
// SandboxTool — Tool trait adapter
// ---------------------------------------------------------------------------

/// A Synaptic [`Tool`] that wraps a [`SandboxBackend`] for agent-driven code execution.
///
/// Accepts JSON arguments `{ "code": "...", "language": "..." }` and returns
/// `{ "stdout": "...", "stderr": "...", "exit_code": N }`.
pub struct SandboxTool<B: SandboxBackend> {
    backend: B,
    limits: ResourceLimits,
}

impl<B: SandboxBackend> SandboxTool<B> {
    /// Create a new `SandboxTool` with the given backend and default resource limits.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            limits: ResourceLimits::default(),
        }
    }

    /// Create a new `SandboxTool` with custom resource limits.
    pub fn with_limits(backend: B, limits: ResourceLimits) -> Self {
        Self { backend, limits }
    }
}

#[async_trait]
impl<B: SandboxBackend + 'static> Tool for SandboxTool<B> {
    fn name(&self) -> &'static str {
        "sandbox_code_executor"
    }

    fn description(&self) -> &'static str {
        "Execute code in an isolated container sandbox. Supports Python, JavaScript, and Bash. \
         Returns stdout, stderr, and exit code."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The code to execute"
                },
                "language": {
                    "type": "string",
                    "enum": ["python", "javascript", "bash"],
                    "description": "The programming language of the code"
                }
            },
            "required": ["code", "language"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let code = args["code"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'code' parameter".to_string()))?;
        let language = args["language"].as_str().unwrap_or("bash");

        let result = self.backend.execute(language, code, &self.limits).await?;

        Ok(json!({
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
        }))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// A mock backend for testing `SandboxTool` without Docker or containers.
    struct MockBackend {
        available: AtomicBool,
        exit_code: i32,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                available: AtomicBool::new(true),
                exit_code: 0,
            }
        }

        fn with_exit_code(mut self, code: i32) -> Self {
            self.exit_code = code;
            self
        }

        fn set_available(&self, available: bool) {
            self.available.store(available, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl SandboxBackend for MockBackend {
        async fn execute(
            &self,
            language: &str,
            code: &str,
            _limits: &ResourceLimits,
        ) -> Result<SandboxResult, SynapticError> {
            Ok(SandboxResult {
                stdout: format!("[{language}] {code}"),
                stderr: String::new(),
                exit_code: self.exit_code,
            })
        }

        async fn is_available(&self) -> bool {
            self.available.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn sandbox_result_defaults() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_mb, 512);
        assert_eq!(limits.cpu_count, 1.0);
        assert_eq!(limits.timeout_secs, 30);
        assert!(!limits.network);
    }

    #[test]
    fn sandbox_result_serialize() {
        let result = SandboxResult {
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["stdout"], "hello");
        assert_eq!(json["exit_code"], 0);
    }

    #[test]
    fn tool_name_and_description() {
        let tool = SandboxTool::new(MockBackend::new());
        assert_eq!(tool.name(), "sandbox_code_executor");
        assert!(tool.description().contains("sandbox"));
    }

    #[test]
    fn tool_parameters_schema() {
        let tool = SandboxTool::new(MockBackend::new());
        let params = tool.parameters().unwrap();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["code"].is_object());
        assert!(params["properties"]["language"].is_object());
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&json!("code")));
        assert!(required.contains(&json!("language")));
    }

    #[tokio::test]
    async fn tool_call_success() {
        let tool = SandboxTool::new(MockBackend::new());
        let result = tool
            .call(json!({"code": "print(42)", "language": "python"}))
            .await
            .unwrap();
        assert_eq!(result["stdout"], "[python] print(42)");
        assert_eq!(result["stderr"], "");
        assert_eq!(result["exit_code"], 0);
    }

    #[tokio::test]
    async fn tool_call_missing_code() {
        let tool = SandboxTool::new(MockBackend::new());
        let result = tool.call(json!({"language": "python"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("code"));
    }

    #[tokio::test]
    async fn tool_call_default_language() {
        let tool = SandboxTool::new(MockBackend::new());
        let result = tool.call(json!({"code": "echo hi"})).await.unwrap();
        // Default language is "bash"
        assert_eq!(result["stdout"], "[bash] echo hi");
    }

    #[tokio::test]
    async fn tool_call_nonzero_exit() {
        let tool = SandboxTool::new(MockBackend::new().with_exit_code(1));
        let result = tool
            .call(json!({"code": "fail", "language": "bash"}))
            .await
            .unwrap();
        assert_eq!(result["exit_code"], 1);
    }

    #[tokio::test]
    async fn mock_backend_availability() {
        let backend = Arc::new(MockBackend::new());
        assert!(backend.is_available().await);
        backend.set_available(false);
        assert!(!backend.is_available().await);
    }

    #[test]
    fn custom_limits() {
        let limits = ResourceLimits {
            memory_mb: 1024,
            cpu_count: 2.0,
            timeout_secs: 60,
            network: true,
        };
        let tool = SandboxTool::with_limits(MockBackend::new(), limits);
        assert_eq!(tool.limits.memory_mb, 1024);
        assert_eq!(tool.limits.cpu_count, 2.0);
        assert_eq!(tool.limits.timeout_secs, 60);
        assert!(tool.limits.network);
    }
}
