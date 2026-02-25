//! Apple Container sandbox backend (macOS only).
//!
//! Uses the `container` CLI (available on macOS 26+) to run code in lightweight
//! Apple containers. Requires the `apple-container` feature flag and must be
//! compiled for macOS (`target_os = "macos"`).

#[cfg(all(target_os = "macos", feature = "apple-container"))]
mod inner {
    use std::process::Stdio;

    use async_trait::async_trait;
    use synaptic_core::SynapticError;
    use tokio::io::AsyncReadExt;

    use crate::{ResourceLimits, SandboxBackend, SandboxResult};

    /// Apple Container sandbox backend.
    ///
    /// Shells out to the macOS `container` CLI to execute code in lightweight
    /// Apple containers.
    ///
    /// # Requirements
    ///
    /// - macOS 26 or later with the `container` CLI installed.
    /// - A pre-built container bundle at the configured `bundle_path`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use synaptic_sandbox::apple::AppleContainerSandbox;
    /// use synaptic_sandbox::{SandboxBackend, ResourceLimits};
    ///
    /// let sandbox = AppleContainerSandbox::new("/path/to/bundle");
    /// let result = sandbox.execute("bash", "echo hello", &ResourceLimits::default()).await?;
    /// ```
    #[derive(Debug, Clone)]
    pub struct AppleContainerSandbox {
        /// Path to the container bundle directory.
        bundle_path: String,
    }

    impl AppleContainerSandbox {
        /// Create a new Apple Container sandbox with the given bundle path.
        pub fn new(bundle_path: impl Into<String>) -> Self {
            Self {
                bundle_path: bundle_path.into(),
            }
        }
    }

    #[async_trait]
    impl SandboxBackend for AppleContainerSandbox {
        async fn execute(
            &self,
            _language: &str,
            code: &str,
            limits: &ResourceLimits,
        ) -> Result<SandboxResult, SynapticError> {
            let mut cmd = tokio::process::Command::new("container");
            cmd.args(["run", "--bundle", &self.bundle_path, "--", "sh", "-c", code])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            tracing::debug!("executing apple container sandbox");

            let timeout_dur = std::time::Duration::from_secs(limits.timeout_secs);

            let result = tokio::time::timeout(timeout_dur, async {
                let mut child = cmd
                    .spawn()
                    .map_err(|e| SynapticError::Tool(format!("failed to spawn container: {e}")))?;

                let mut stdout_buf = String::new();
                let mut stderr_buf = String::new();

                if let Some(ref mut stdout) = child.stdout {
                    stdout.read_to_string(&mut stdout_buf).await.map_err(|e| {
                        SynapticError::Tool(format!("failed to read container stdout: {e}"))
                    })?;
                }
                if let Some(ref mut stderr) = child.stderr {
                    stderr.read_to_string(&mut stderr_buf).await.map_err(|e| {
                        SynapticError::Tool(format!("failed to read container stderr: {e}"))
                    })?;
                }

                let status = child.wait().await.map_err(|e| {
                    SynapticError::Tool(format!("failed to wait on container: {e}"))
                })?;

                Ok(SandboxResult {
                    stdout: stdout_buf,
                    stderr: stderr_buf,
                    exit_code: status.code().unwrap_or(-1),
                })
            })
            .await;

            match result {
                Ok(inner) => inner,
                Err(_) => Err(SynapticError::Timeout(format!(
                    "apple container execution exceeded {}s timeout",
                    limits.timeout_secs
                ))),
            }
        }

        async fn is_available(&self) -> bool {
            let cmd = tokio::process::Command::new("container")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;

            matches!(cmd, Ok(s) if s.success())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn new_sets_bundle_path() {
            let sandbox = AppleContainerSandbox::new("/my/bundle");
            assert_eq!(sandbox.bundle_path, "/my/bundle");
        }
    }
}

#[cfg(all(target_os = "macos", feature = "apple-container"))]
pub use inner::AppleContainerSandbox;
