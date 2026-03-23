use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;

use super::fs_bridge::{FsBridge, MountMapping};
use super::provider::*;
use super::types::*;
use super::validate::validate_sandbox_security;
use crate::backend::{Backend, DirEntry, ExecResult, GrepOutputMode};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerProviderConfig {
    #[serde(default = "default_image")]
    pub image: String,
    #[serde(default = "default_prefix")]
    pub container_prefix: String,
    #[serde(default = "default_tmpfs")]
    pub tmpfs_mounts: Vec<String>,
    pub user: Option<String>,
}

fn default_image() -> String {
    "synapse-sandbox:bookworm-slim".to_string()
}
fn default_prefix() -> String {
    "synapse-sbx-".to_string()
}
fn default_tmpfs() -> Vec<String> {
    vec!["/tmp".into(), "/var/tmp".into(), "/run".into()]
}

impl Default for DockerProviderConfig {
    fn default() -> Self {
        Self {
            image: default_image(),
            container_prefix: default_prefix(),
            tmpfs_mounts: default_tmpfs(),
            user: None,
        }
    }
}

// ---------------------------------------------------------------------------
// DockerBackend — Backend via `docker exec`
// ---------------------------------------------------------------------------

struct DockerBackend {
    container_id: String,
    work_dir: String,
}

impl DockerBackend {
    fn new(container_id: String, work_dir: String) -> Self {
        Self {
            container_id,
            work_dir,
        }
    }

    async fn docker_exec(&self, cmd: &str) -> Result<ExecResult, SynapticError> {
        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-w",
                &self.work_dir,
                &self.container_id,
                "sh",
                "-c",
                cmd,
            ])
            .output()
            .await
            .map_err(|e| SynapticError::Tool(format!("docker exec failed: {e}")))?;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}

#[async_trait]
impl Backend for DockerBackend {
    async fn ls(&self, path: &str) -> Result<Vec<DirEntry>, SynapticError> {
        let cmd = format!("ls -la --time-style=+%s {}", shell_escape(path));
        let result = self.docker_exec(&cmd).await?;

        if result.exit_code != 0 {
            return Err(SynapticError::Tool(format!("ls failed: {}", result.stderr)));
        }

        let mut entries = Vec::new();
        for line in result.stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }
            let perms = parts[0];
            let size: u64 = parts[4].parse().unwrap_or(0);
            let name = parts[6..].join(" ");

            if name == "." || name == ".." {
                continue;
            }

            entries.push(DirEntry {
                name,
                is_dir: perms.starts_with('d'),
                size: Some(size),
            });
        }

        Ok(entries)
    }

    async fn read_file(
        &self,
        path: &str,
        offset: usize,
        limit: usize,
    ) -> Result<String, SynapticError> {
        let start = offset + 1;
        let end = offset + limit;
        let cmd = format!("sed -n '{start},{end}p' {}", shell_escape(path));
        let result = self.docker_exec(&cmd).await?;

        if result.exit_code != 0 {
            return Err(SynapticError::Tool(format!(
                "read failed: {}",
                result.stderr
            )));
        }

        Ok(result.stdout)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), SynapticError> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let mkdir_cmd = format!("mkdir -p {}", shell_escape(&parent.to_string_lossy()));
            self.docker_exec(&mkdir_cmd).await?;
        }

        let mut child = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-i",
                "-w",
                &self.work_dir,
                &self.container_id,
                "sh",
                "-c",
                &format!("cat > {}", shell_escape(path)),
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SynapticError::Tool(format!("docker exec failed: {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(content.as_bytes())
                .await
                .map_err(|e| SynapticError::Tool(format!("write failed: {e}")))?;
        }

        let status = child
            .wait()
            .await
            .map_err(|e| SynapticError::Tool(format!("docker exec wait: {e}")))?;

        if !status.success() {
            return Err(SynapticError::Tool("write_file failed in container".into()));
        }

        Ok(())
    }

    async fn edit_file(
        &self,
        path: &str,
        old_text: &str,
        new_text: &str,
        _replace_all: bool,
    ) -> Result<(), SynapticError> {
        let content = self.read_file(path, 0, 100_000).await?;
        if !content.contains(old_text) {
            return Err(SynapticError::Tool(format!(
                "old_string not found in {path}"
            )));
        }
        let new_content = content.replacen(old_text, new_text, 1);
        self.write_file(path, &new_content).await
    }

    async fn glob(&self, pattern: &str, base: &str) -> Result<Vec<String>, SynapticError> {
        let cmd = format!(
            "find {} -type f -name '{}' 2>/dev/null | sort",
            shell_escape(base),
            pattern.replace("**", "*")
        );
        let result = self.docker_exec(&cmd).await?;

        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    async fn grep(
        &self,
        pattern: &str,
        path: Option<&str>,
        file_glob: Option<&str>,
        output_mode: GrepOutputMode,
    ) -> Result<String, SynapticError> {
        let dir = path.unwrap_or(".");
        let mut cmd = String::from("grep -r");

        match output_mode {
            GrepOutputMode::FilesWithMatches => cmd.push_str(" -l"),
            GrepOutputMode::Count => cmd.push_str(" -c"),
            GrepOutputMode::Content => cmd.push_str(" -n"),
        }

        if let Some(glob) = file_glob {
            cmd.push_str(&format!(" --include='{glob}'"));
        }

        cmd.push_str(&format!(" {} {}", shell_escape(pattern), shell_escape(dir)));
        cmd.push_str(" 2>/dev/null");

        let result = self.docker_exec(&cmd).await?;
        Ok(result.stdout)
    }

    async fn execute(
        &self,
        command: &str,
        timeout: Option<Duration>,
    ) -> Result<ExecResult, SynapticError> {
        let cmd = if let Some(dur) = timeout {
            format!("timeout {}s sh -c {}", dur.as_secs(), shell_escape(command))
        } else {
            command.to_string()
        };

        self.docker_exec(&cmd).await
    }

    fn supports_execution(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// DockerProvider — SandboxProvider implementation
// ---------------------------------------------------------------------------

pub struct DockerProvider {
    config: DockerProviderConfig,
}

impl DockerProvider {
    pub fn new(config: DockerProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for DockerProvider {
    fn default() -> Self {
        Self::new(DockerProviderConfig::default())
    }
}

/// Sanitize a scope key for use as a container name component.
fn sanitize_scope_key(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[async_trait]
impl SandboxProvider for DockerProvider {
    fn id(&self) -> &str {
        "docker"
    }

    async fn create(&self, req: SandboxCreateRequest) -> Result<SandboxInstance, SynapticError> {
        // 1. Validate security
        validate_sandbox_security(&req.security, &req.extra_mounts)?;

        let sanitized_key = sanitize_scope_key(&req.scope_key);
        let container_name = format!("{}{sanitized_key}", self.config.container_prefix);
        let container_workspace = PathBuf::from("/workspace");

        // 2. Build `docker run` args
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name.clone(),
        ];

        // Security: cap drop
        for cap in &req.security.cap_drop {
            args.push("--cap-drop".into());
            args.push(cap.clone());
        }

        // Security: read-only root
        if req.security.read_only_root {
            args.push("--read-only".into());
        }

        // Security: network mode
        match &req.security.network_mode {
            NetworkMode::None => {
                args.push("--network".into());
                args.push("none".into());
            }
            NetworkMode::Bridge => {
                args.push("--network".into());
                args.push("bridge".into());
            }
            NetworkMode::Host => {
                // validate_sandbox_security already rejects Host, but be safe
                return Err(SynapticError::Security(
                    "sandbox: host network mode rejected".into(),
                ));
            }
            NetworkMode::Custom(name) => {
                args.push("--network".into());
                args.push(name.clone());
            }
        }

        // Security: seccomp/apparmor
        if let Some(ref profile) = req.security.seccomp_profile {
            args.push("--security-opt".into());
            args.push(format!("seccomp={profile}"));
        }
        if let Some(ref profile) = req.security.apparmor_profile {
            args.push("--security-opt".into());
            args.push(format!("apparmor={profile}"));
        }

        // Resources
        if let Some(ref mem) = req.resources.memory {
            args.push("--memory".into());
            args.push(mem.clone());
        }
        if let Some(ref swap) = req.resources.memory_swap {
            args.push("--memory-swap".into());
            args.push(swap.clone());
        }
        if let Some(cpus) = req.resources.cpus {
            args.push("--cpus".into());
            args.push(cpus.to_string());
        }
        if let Some(pids) = req.resources.pids_limit {
            args.push("--pids-limit".into());
            args.push(pids.to_string());
        }

        // tmpfs mounts
        for tmpfs in &self.config.tmpfs_mounts {
            args.push("--tmpfs".into());
            args.push(tmpfs.clone());
        }

        // User
        if let Some(ref user) = self.config.user {
            args.push("--user".into());
            args.push(user.clone());
        }

        // Workspace mount (based on WorkspaceAccess)
        let mount_read_only = match req.workspace.access {
            WorkspaceAccess::None => {
                // No bind mount — we'll seed via `docker cp` after start
                false // not used, but track for later
            }
            WorkspaceAccess::ReadOnly => {
                let host = req.workspace.host_dir.to_string_lossy().to_string();
                let container = container_workspace.to_string_lossy().to_string();
                args.push("-v".into());
                args.push(format!("{host}:{container}:ro"));
                true
            }
            WorkspaceAccess::ReadWrite => {
                let host = req.workspace.host_dir.to_string_lossy().to_string();
                let container = container_workspace.to_string_lossy().to_string();
                args.push("-v".into());
                args.push(format!("{host}:{container}:rw"));
                false
            }
        };

        // Extra bind mounts
        for mount in &req.extra_mounts {
            let host = mount.host_path.to_string_lossy().to_string();
            let container = mount.container_path.to_string_lossy().to_string();
            let mode = if mount.read_only { "ro" } else { "rw" };
            args.push("-v".into());
            args.push(format!("{host}:{container}:{mode}"));
        }

        // Environment variables
        for (k, v) in &req.env {
            args.push("-e".into());
            args.push(format!("{}={}", k, v));
        }

        // Labels
        args.push("--label".into());
        args.push(format!("synapse.sandbox.scope_key={}", req.scope_key));
        args.push("--label".into());
        args.push("synapse.sandbox.provider=docker".into());

        // Image + entrypoint
        args.push(self.config.image.clone());
        args.push("sleep".into());
        args.push("infinity".into());

        // 3. Run container
        let output = tokio::process::Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| SynapticError::Tool(format!("docker run failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SynapticError::Tool(format!(
                "docker run failed (exit {}): {stderr}",
                output.status.code().unwrap_or(-1)
            )));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // 4. For WorkspaceAccess::None, seed via docker cp
        if req.workspace.access == WorkspaceAccess::None {
            // Create workspace dir in container
            let mkdir_out = tokio::process::Command::new("docker")
                .args([
                    "exec",
                    &container_id,
                    "mkdir",
                    "-p",
                    &container_workspace.to_string_lossy(),
                ])
                .output()
                .await
                .map_err(|e| SynapticError::Tool(format!("docker exec mkdir failed: {e}")))?;

            if !mkdir_out.status.success() {
                // Clean up on failure
                let _ = tokio::process::Command::new("docker")
                    .args(["rm", "-f", &container_id])
                    .output()
                    .await;
                return Err(SynapticError::Tool(
                    "failed to create workspace directory in container".into(),
                ));
            }

            // Copy host workspace content into container
            let src = format!("{}/.  ", req.workspace.host_dir.to_string_lossy());
            let dst = format!("{container_id}:{}", container_workspace.to_string_lossy());

            let cp_out = tokio::process::Command::new("docker")
                .args(["cp", src.trim(), &dst])
                .output()
                .await
                .map_err(|e| SynapticError::Tool(format!("docker cp failed: {e}")))?;

            if !cp_out.status.success() {
                let _ = tokio::process::Command::new("docker")
                    .args(["rm", "-f", &container_id])
                    .output()
                    .await;
                let stderr = String::from_utf8_lossy(&cp_out.stderr);
                return Err(SynapticError::Tool(format!("docker cp failed: {stderr}")));
            }
        }

        // 5. Run setup command if present
        if let Some(ref setup_cmd) = req.setup_command {
            let setup_out = tokio::process::Command::new("docker")
                .args([
                    "exec",
                    "-w",
                    &container_workspace.to_string_lossy(),
                    &container_id,
                    "sh",
                    "-c",
                    setup_cmd,
                ])
                .output()
                .await
                .map_err(|e| SynapticError::Tool(format!("setup command failed: {e}")))?;

            if !setup_out.status.success() {
                let stderr = String::from_utf8_lossy(&setup_out.stderr);
                tracing::warn!(
                    container_id = %container_id,
                    stderr = %stderr,
                    "sandbox setup command exited with non-zero status"
                );
            }
        }

        // 6. Create DockerBackend + FsBridge
        let backend = Arc::new(DockerBackend::new(
            container_id.clone(),
            container_workspace.to_string_lossy().to_string(),
        ));

        // Build mount mappings for FsBridge
        let mut mount_mappings = vec![];
        if req.workspace.access != WorkspaceAccess::None {
            mount_mappings.push(MountMapping {
                host_path: req.workspace.host_dir.clone(),
                container_path: container_workspace.clone(),
                read_only: mount_read_only,
            });
        } else {
            // Copied workspace — treat as read-write within container
            mount_mappings.push(MountMapping {
                host_path: req.workspace.host_dir.clone(),
                container_path: container_workspace.clone(),
                read_only: false,
            });
        }

        for mount in &req.extra_mounts {
            mount_mappings.push(MountMapping {
                host_path: mount.host_path.clone(),
                container_path: mount.container_path.clone(),
                read_only: mount.read_only,
            });
        }

        let mut allowed_roots: Vec<PathBuf> = vec![container_workspace.clone()];
        for mount in &req.extra_mounts {
            allowed_roots.push(mount.container_path.clone());
        }
        // Allow tmpfs paths
        for tmpfs in &self.config.tmpfs_mounts {
            allowed_roots.push(PathBuf::from(tmpfs));
        }

        let bridge = FsBridge::new(backend, mount_mappings, allowed_roots);

        let now = Utc::now();
        let info = SandboxInstanceInfo {
            runtime_id: container_id.clone(),
            provider_id: "docker".into(),
            runtime_label: container_name,
            scope_key: req.scope_key,
            image: Some(self.config.image.clone()),
            created_at: now,
            last_used_at: now,
        };

        Ok(SandboxInstance {
            runtime_id: container_id,
            backend: Arc::new(bridge),
            info,
        })
    }

    async fn destroy(&self, runtime_id: &str) -> Result<(), SynapticError> {
        // Kill first, then force remove
        let _ = tokio::process::Command::new("docker")
            .args(["kill", runtime_id])
            .output()
            .await;

        let output = tokio::process::Command::new("docker")
            .args(["rm", "-f", runtime_id])
            .output()
            .await
            .map_err(|e| SynapticError::Tool(format!("docker rm failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SynapticError::Tool(format!(
                "docker rm -f failed: {stderr}"
            )));
        }

        Ok(())
    }

    async fn status(&self, runtime_id: &str) -> Result<SandboxStatus, SynapticError> {
        let output = tokio::process::Command::new("docker")
            .args(["inspect", "--format", "{{.State.Status}}", runtime_id])
            .output()
            .await
            .map_err(|e| SynapticError::Tool(format!("docker inspect failed: {e}")))?;

        if !output.status.success() {
            return Ok(SandboxStatus::NotFound);
        }

        let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        match status_str.as_str() {
            "running" => Ok(SandboxStatus::Running),
            "created" | "restarting" | "paused" => Ok(SandboxStatus::Running),
            "exited" | "dead" | "removing" => Ok(SandboxStatus::Stopped),
            _ => Ok(SandboxStatus::Stopped),
        }
    }

    async fn list(&self) -> Result<Vec<SandboxInstanceInfo>, SynapticError> {
        let filter = format!("name={}", self.config.container_prefix);
        let format_str =
            "{{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Image}}\t{{.Label \"synapse.sandbox.scope_key\"}}\t{{.CreatedAt}}";

        let output = tokio::process::Command::new("docker")
            .args(["ps", "-a", "--filter", &filter, "--format", format_str])
            .output()
            .await
            .map_err(|e| SynapticError::Tool(format!("docker ps failed: {e}")))?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut instances = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 6 {
                continue;
            }

            let runtime_id = parts[0].to_string();
            let runtime_label = parts[1].to_string();
            let _status = parts[2]; // human-readable status
            let image = parts[3].to_string();
            let scope_key = parts[4].to_string();
            let created_str = parts[5];

            // Parse created_at — best-effort, fallback to now
            let created_at =
                chrono::NaiveDateTime::parse_from_str(created_str, "%Y-%m-%d %H:%M:%S %z")
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(|_| Utc::now());

            instances.push(SandboxInstanceInfo {
                runtime_id,
                provider_id: "docker".into(),
                runtime_label,
                scope_key,
                image: Some(image),
                created_at,
                last_used_at: created_at,
            });
        }

        Ok(instances)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_scope_key() {
        assert_eq!(sanitize_scope_key("user-123"), "user-123");
        assert_eq!(sanitize_scope_key("user@host:path"), "user-host-path");
        assert_eq!(sanitize_scope_key("hello world"), "hello-world");
        assert_eq!(sanitize_scope_key("abc123DEF"), "abc123DEF");
    }

    #[test]
    fn test_shell_escape_basic() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_default_config() {
        let config = DockerProviderConfig::default();
        assert_eq!(config.image, "synapse-sandbox:bookworm-slim");
        assert_eq!(config.container_prefix, "synapse-sbx-");
        assert_eq!(config.tmpfs_mounts.len(), 3);
        assert!(config.user.is_none());
    }

    #[test]
    fn test_provider_id() {
        let provider = DockerProvider::default();
        assert_eq!(provider.id(), "docker");
    }
}
