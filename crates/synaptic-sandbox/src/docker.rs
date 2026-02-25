//! Docker-based sandbox backend.
//!
//! Shells out to `docker run` to execute code in isolated containers with
//! resource limits (memory, CPU, network, timeout).

use std::collections::HashMap;
use std::process::Stdio;

use async_trait::async_trait;
use synaptic_core::SynapticError;
use tokio::io::AsyncReadExt;

use crate::{ResourceLimits, SandboxBackend, SandboxResult};

/// Docker container sandbox backend.
///
/// Executes code inside ephemeral `docker run --rm` containers with configurable
/// resource limits and per-language images.
///
/// # Default images
///
/// | Language     | Image               |
/// |-------------|----------------------|
/// | python      | python:3.12-slim     |
/// | javascript  | node:22-slim         |
/// | bash        | alpine:latest        |
///
/// # Example
///
/// ```rust,ignore
/// use synaptic_sandbox::docker::DockerSandbox;
/// use synaptic_sandbox::{SandboxBackend, ResourceLimits};
///
/// let sandbox = DockerSandbox::default()
///     .with_image("rust", "rust:1.88-slim");
/// let result = sandbox.execute("python", "print('hello')", &ResourceLimits::default()).await?;
/// assert_eq!(result.exit_code, 0);
/// ```
#[derive(Debug, Clone)]
pub struct DockerSandbox {
    /// Docker host URI (e.g. `unix:///var/run/docker.sock`).
    docker_host: Option<String>,
    /// Default image used when no language-specific image is configured.
    default_image: String,
    /// Language-to-image mapping.
    images: HashMap<String, String>,
}

impl Default for DockerSandbox {
    fn default() -> Self {
        let mut images = HashMap::new();
        images.insert("python".to_string(), "python:3.12-slim".to_string());
        images.insert("javascript".to_string(), "node:22-slim".to_string());
        images.insert("bash".to_string(), "alpine:latest".to_string());
        Self {
            docker_host: None,
            default_image: "alpine:latest".to_string(),
            images,
        }
    }
}

impl DockerSandbox {
    /// Set a custom Docker host URI.
    pub fn with_docker_host(mut self, host: impl Into<String>) -> Self {
        self.docker_host = Some(host.into());
        self
    }

    /// Register a custom image for a language.
    pub fn with_image(mut self, language: impl Into<String>, image: impl Into<String>) -> Self {
        self.images.insert(language.into(), image.into());
        self
    }

    /// Look up the container image for the given language.
    pub fn image_for(&self, language: &str) -> &str {
        self.images
            .get(language)
            .map(|s| s.as_str())
            .unwrap_or(&self.default_image)
    }

    /// Build the `docker run` command arguments (without the leading `docker`).
    pub fn build_args(&self, language: &str, code: &str, limits: &ResourceLimits) -> Vec<String> {
        let image = self.image_for(language);
        let mut args = vec!["run".to_string(), "--rm".to_string()];

        // Network isolation
        if !limits.network {
            args.push("--network=none".to_string());
        }

        // Memory limit
        args.push(format!("--memory={}m", limits.memory_mb));

        // CPU limit
        args.push(format!("--cpus={}", limits.cpu_count));

        // Image
        args.push(image.to_string());

        // Command: run code via shell
        let shell_cmd = get_shell_cmd(language, code);
        args.extend(shell_cmd);

        args
    }
}

#[async_trait]
impl SandboxBackend for DockerSandbox {
    async fn execute(
        &self,
        language: &str,
        code: &str,
        limits: &ResourceLimits,
    ) -> Result<SandboxResult, SynapticError> {
        let args = self.build_args(language, code, limits);

        let mut cmd = tokio::process::Command::new("docker");

        if let Some(ref host) = self.docker_host {
            cmd.env("DOCKER_HOST", host);
        }

        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!(language, "executing docker sandbox");

        let timeout_dur = std::time::Duration::from_secs(limits.timeout_secs);

        let result = tokio::time::timeout(timeout_dur, async {
            let mut child = cmd
                .spawn()
                .map_err(|e| SynapticError::Tool(format!("failed to spawn docker: {e}")))?;

            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();

            if let Some(ref mut stdout) = child.stdout {
                stdout.read_to_string(&mut stdout_buf).await.map_err(|e| {
                    SynapticError::Tool(format!("failed to read docker stdout: {e}"))
                })?;
            }
            if let Some(ref mut stderr) = child.stderr {
                stderr.read_to_string(&mut stderr_buf).await.map_err(|e| {
                    SynapticError::Tool(format!("failed to read docker stderr: {e}"))
                })?;
            }

            let status = child
                .wait()
                .await
                .map_err(|e| SynapticError::Tool(format!("failed to wait on docker: {e}")))?;

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
                "docker execution exceeded {}s timeout",
                limits.timeout_secs
            ))),
        }
    }

    async fn is_available(&self) -> bool {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(ref host) = self.docker_host {
            cmd.env("DOCKER_HOST", host);
        }

        matches!(cmd.status().await, Ok(s) if s.success())
    }
}

/// Build the shell command to execute code in the given language.
fn get_shell_cmd(language: &str, code: &str) -> Vec<String> {
    match language {
        "python" => vec!["python3".to_string(), "-c".to_string(), code.to_string()],
        "javascript" | "js" => vec!["node".to_string(), "-e".to_string(), code.to_string()],
        _ => vec!["sh".to_string(), "-c".to_string(), code.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_images() {
        let sandbox = DockerSandbox::default();
        assert_eq!(sandbox.image_for("python"), "python:3.12-slim");
        assert_eq!(sandbox.image_for("javascript"), "node:22-slim");
        assert_eq!(sandbox.image_for("bash"), "alpine:latest");
    }

    #[test]
    fn unknown_language_uses_default_image() {
        let sandbox = DockerSandbox::default();
        assert_eq!(sandbox.image_for("ruby"), "alpine:latest");
    }

    #[test]
    fn custom_image() {
        let sandbox = DockerSandbox::default().with_image("rust", "rust:1.88-slim");
        assert_eq!(sandbox.image_for("rust"), "rust:1.88-slim");
    }

    #[test]
    fn docker_host_builder() {
        let sandbox = DockerSandbox::default().with_docker_host("tcp://localhost:2375");
        assert_eq!(sandbox.docker_host.as_deref(), Some("tcp://localhost:2375"));
    }

    #[test]
    fn build_args_network_disabled() {
        let sandbox = DockerSandbox::default();
        let limits = ResourceLimits::default();
        let args = sandbox.build_args("python", "print('hi')", &limits);

        assert!(args.contains(&"--network=none".to_string()));
        assert!(args.contains(&"--rm".to_string()));
        assert!(args.contains(&"python:3.12-slim".to_string()));
        assert!(args.contains(&format!("--memory={}m", limits.memory_mb)));
        assert!(args.contains(&format!("--cpus={}", limits.cpu_count)));
    }

    #[test]
    fn build_args_network_enabled() {
        let sandbox = DockerSandbox::default();
        let limits = ResourceLimits {
            network: true,
            ..Default::default()
        };
        let args = sandbox.build_args("bash", "echo hi", &limits);

        assert!(!args.contains(&"--network=none".to_string()));
    }

    #[test]
    fn build_args_contains_code() {
        let sandbox = DockerSandbox::default();
        let limits = ResourceLimits::default();
        let args = sandbox.build_args("python", "print(42)", &limits);

        // The code should appear in the args
        assert!(args.contains(&"print(42)".to_string()));
        assert!(args.contains(&"python3".to_string()));
        assert!(args.contains(&"-c".to_string()));
    }

    #[test]
    fn shell_cmd_javascript() {
        let cmd = get_shell_cmd("javascript", "console.log(1)");
        assert_eq!(cmd[0], "node");
        assert_eq!(cmd[1], "-e");
        assert_eq!(cmd[2], "console.log(1)");
    }

    #[test]
    fn shell_cmd_bash_fallback() {
        let cmd = get_shell_cmd("unknown", "echo hi");
        assert_eq!(cmd[0], "sh");
        assert_eq!(cmd[1], "-c");
    }
}
