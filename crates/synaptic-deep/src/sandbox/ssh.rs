//! SSH-based sandbox provider for remote execution.
//!
//! All file and command operations are forwarded to a remote host via SSH.
//! Isolation relies on OS-level user permissions on the remote host rather
//! than container-level security controls.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;
use tokio::process::Command;

use super::fs_bridge::{FsBridge, MountMapping};
use super::provider::{SandboxCreateRequest, SandboxInstance, SandboxInstanceInfo, SandboxStatus};
use super::SandboxProvider;
use crate::backend::{Backend, DirEntry, ExecResult, GrepOutputMode};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for the SSH sandbox provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshProviderConfig {
    /// SSH target in `user@host:port` or `user@host` format.
    pub target: String,
    /// Path to the SSH identity (private key) file.
    pub identity_file: Option<PathBuf>,
    /// Whether to enforce strict host key checking (default: true).
    #[serde(default = "default_true")]
    pub strict_host_key_checking: bool,
    /// Root directory on the remote host under which workspaces are created.
    pub workspace_root: PathBuf,
    /// Workspace synchronisation mode.
    #[serde(default)]
    pub workspace_mode: SshWorkspaceMode,
}

fn default_true() -> bool {
    true
}

/// How workspaces are synchronised between local and remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SshWorkspaceMode {
    /// Local filesystem is canonical; files are synced **to** the remote.
    #[default]
    Mirror,
    /// Remote filesystem is canonical; changes stay remote.
    Remote,
}

// ---------------------------------------------------------------------------
// Shell escaping helper
// ---------------------------------------------------------------------------

/// Shell-escape a string for safe inclusion in a single-quoted SSH command.
/// This prevents injection when user-provided values are interpolated.
fn shell_escape(s: &str) -> String {
    // Wrap in single quotes and escape any embedded single quotes
    // 'foo'\''bar' => foo'bar
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ---------------------------------------------------------------------------
// SSH helpers
// ---------------------------------------------------------------------------

/// Parse the target string into (host_part, port).
/// Accepts `user@host:port` or `user@host` (port defaults to 22).
fn parse_target(target: &str) -> (&str, Option<u16>) {
    if let Some(idx) = target.rfind(':') {
        let after = &target[idx + 1..];
        if let Ok(port) = after.parse::<u16>() {
            return (&target[..idx], Some(port));
        }
    }
    (target, None)
}

/// Build common SSH argument list from the provider config.
fn ssh_base_args(config: &SshProviderConfig) -> Vec<String> {
    let (host, port) = parse_target(&config.target);
    let mut args: Vec<String> = Vec::new();

    if let Some(p) = port {
        args.push("-p".into());
        args.push(p.to_string());
    }

    if let Some(ref key) = config.identity_file {
        args.push("-i".into());
        args.push(key.to_string_lossy().to_string());
    }

    let check = if config.strict_host_key_checking {
        "yes"
    } else {
        "no"
    };
    args.push("-o".into());
    args.push(format!("StrictHostKeyChecking={check}"));

    // Disable interactive prompts
    args.push("-o".into());
    args.push("BatchMode=yes".into());

    args.push(host.to_string());
    args
}

/// Run an SSH command and return (stdout, stderr, exit_code).
async fn ssh_exec(
    config: &SshProviderConfig,
    remote_cmd: &str,
) -> Result<(String, String, i32), SynapticError> {
    let mut args = ssh_base_args(config);
    args.push(remote_cmd.to_string());

    let output = Command::new("ssh")
        .args(&args)
        .output()
        .await
        .map_err(|e| SynapticError::Tool(format!("ssh exec failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((stdout, stderr, code))
}

/// Run an SSH command, piping `stdin_data` into the remote process.
async fn ssh_exec_stdin(
    config: &SshProviderConfig,
    remote_cmd: &str,
    stdin_data: &[u8],
) -> Result<(String, String, i32), SynapticError> {
    let mut args = ssh_base_args(config);
    args.push(remote_cmd.to_string());

    let mut child = Command::new("ssh")
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| SynapticError::Tool(format!("ssh spawn failed: {e}")))?;

    // Write stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(stdin_data)
            .await
            .map_err(|e| SynapticError::Tool(format!("ssh stdin write failed: {e}")))?;
        // Drop to close stdin
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| SynapticError::Tool(format!("ssh wait failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((stdout, stderr, code))
}

// ---------------------------------------------------------------------------
// SshBackend
// ---------------------------------------------------------------------------

/// Backend implementation that executes all operations on a remote host via SSH.
struct SshBackend {
    config: SshProviderConfig,
    /// The remote working directory for this sandbox instance.
    remote_dir: PathBuf,
}

#[async_trait]
impl Backend for SshBackend {
    async fn ls(&self, path: &str) -> Result<Vec<DirEntry>, SynapticError> {
        let escaped = shell_escape(path);
        let cmd = format!("ls -la {escaped} 2>/dev/null || true");
        let (stdout, _stderr, _code) = ssh_exec(&self.config, &cmd).await?;

        let mut entries = Vec::new();
        for line in stdout.lines().skip(1) {
            // Skip the "total N" line
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }
            let perms = parts[0];
            let is_dir = perms.starts_with('d');
            let size = parts[4].parse::<u64>().ok();
            // Name is the last field (may contain spaces, but we take the last element)
            let name = parts.last().unwrap_or(&"").to_string();
            if name == "." || name == ".." {
                continue;
            }
            entries.push(DirEntry { name, is_dir, size });
        }
        Ok(entries)
    }

    async fn read_file(
        &self,
        path: &str,
        offset: usize,
        limit: usize,
    ) -> Result<String, SynapticError> {
        let escaped = shell_escape(path);
        let start = offset + 1; // sed is 1-based
        let end = offset + limit;
        let cmd = format!("sed -n '{start},{end}p' {escaped}");
        let (stdout, stderr, code) = ssh_exec(&self.config, &cmd).await?;
        if code != 0 {
            return Err(SynapticError::Tool(format!(
                "ssh read_file failed (exit {code}): {stderr}"
            )));
        }
        Ok(stdout)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), SynapticError> {
        let escaped = shell_escape(path);
        // Ensure parent directory exists
        let cmd = format!("mkdir -p \"$(dirname {escaped})\" && cat > {escaped}");
        let (_stdout, stderr, code) =
            ssh_exec_stdin(&self.config, &cmd, content.as_bytes()).await?;
        if code != 0 {
            return Err(SynapticError::Tool(format!(
                "ssh write_file failed (exit {code}): {stderr}"
            )));
        }
        Ok(())
    }

    async fn edit_file(
        &self,
        path: &str,
        old_text: &str,
        new_text: &str,
        replace_all: bool,
    ) -> Result<(), SynapticError> {
        // Read the full file
        let escaped = shell_escape(path);
        let (content, stderr, code) = ssh_exec(&self.config, &format!("cat {escaped}")).await?;
        if code != 0 {
            return Err(SynapticError::Tool(format!(
                "ssh edit_file read failed (exit {code}): {stderr}"
            )));
        }

        let new_content = if replace_all {
            content.replace(old_text, new_text)
        } else {
            content.replacen(old_text, new_text, 1)
        };

        if new_content == content {
            return Err(SynapticError::Tool(
                "edit_file: old_text not found in file".into(),
            ));
        }

        self.write_file(path, &new_content).await
    }

    async fn glob(&self, pattern: &str, base: &str) -> Result<Vec<String>, SynapticError> {
        let escaped_base = shell_escape(base);
        let escaped_pattern = shell_escape(pattern);
        let cmd =
            format!("find {escaped_base} -type f -name {escaped_pattern} 2>/dev/null || true");
        let (stdout, _stderr, _code) = ssh_exec(&self.config, &cmd).await?;
        let files: Vec<String> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
        Ok(files)
    }

    async fn grep(
        &self,
        pattern: &str,
        path: Option<&str>,
        file_glob: Option<&str>,
        output_mode: GrepOutputMode,
    ) -> Result<String, SynapticError> {
        let escaped_pattern = shell_escape(pattern);
        let target = match path {
            Some(p) => shell_escape(p),
            None => shell_escape(self.remote_dir.to_string_lossy().as_ref()),
        };

        let mode_flag = match output_mode {
            GrepOutputMode::FilesWithMatches => "-l",
            GrepOutputMode::Count => "-c",
            GrepOutputMode::Content => "-n",
        };

        let include = match file_glob {
            Some(g) => format!(" --include={}", shell_escape(g)),
            None => String::new(),
        };

        let cmd =
            format!("grep -r {mode_flag}{include} {escaped_pattern} {target} 2>/dev/null || true");
        let (stdout, _stderr, _code) = ssh_exec(&self.config, &cmd).await?;
        Ok(stdout)
    }

    async fn execute(
        &self,
        command: &str,
        timeout: Option<Duration>,
    ) -> Result<ExecResult, SynapticError> {
        let remote_dir_escaped = shell_escape(self.remote_dir.to_string_lossy().as_ref());
        let wrapped = match timeout {
            Some(t) => {
                let secs = t.as_secs();
                format!(
                    "cd {remote_dir_escaped} && timeout {secs} sh -c {}",
                    shell_escape(command)
                )
            }
            None => {
                format!("cd {remote_dir_escaped} && sh -c {}", shell_escape(command))
            }
        };
        let (stdout, stderr, code) = ssh_exec(&self.config, &wrapped).await?;
        Ok(ExecResult {
            stdout,
            stderr,
            exit_code: code,
        })
    }

    fn supports_execution(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// SshProvider
// ---------------------------------------------------------------------------

/// Sandbox provider that creates isolated workspaces on a remote host via SSH.
///
/// Docker-specific security fields (`cap_drop`, `seccomp`, `apparmor`,
/// `read_only_root`) in `SandboxSecurityConfig` are ignored. Isolation relies
/// on OS-level user permissions on the remote host.
pub struct SshProvider {
    config: SshProviderConfig,
}

impl SshProvider {
    /// Create a new SSH provider with the given config.
    pub fn new(config: SshProviderConfig) -> Self {
        Self { config }
    }

    /// Build the rsync argument list for syncing a local dir to the remote.
    fn rsync_args(&self, local_dir: &str, remote_dir: &str) -> Vec<String> {
        let (host, port) = parse_target(&self.config.target);
        let mut args = vec!["-az".to_string()];

        // Build the ssh command for rsync to use
        let mut ssh_cmd = "ssh".to_string();
        if let Some(p) = port {
            ssh_cmd.push_str(&format!(" -p {p}"));
        }
        if let Some(ref key) = self.config.identity_file {
            ssh_cmd.push_str(&format!(" -i {}", key.to_string_lossy()));
        }
        let check = if self.config.strict_host_key_checking {
            "yes"
        } else {
            "no"
        };
        ssh_cmd.push_str(&format!(
            " -o StrictHostKeyChecking={check} -o BatchMode=yes"
        ));
        args.push("-e".into());
        args.push(ssh_cmd);

        // Ensure trailing slash on source so contents are synced, not the dir itself
        let src = if local_dir.ends_with('/') {
            local_dir.to_string()
        } else {
            format!("{local_dir}/")
        };
        args.push(src);
        args.push(format!("{host}:{remote_dir}/"));
        args
    }
}

#[async_trait]
impl SandboxProvider for SshProvider {
    fn id(&self) -> &str {
        "ssh"
    }

    async fn create(&self, req: SandboxCreateRequest) -> Result<SandboxInstance, SynapticError> {
        let scope = &req.scope_key;
        let remote_dir = self
            .config
            .workspace_root
            .join(scope)
            .to_string_lossy()
            .to_string();
        let remote_dir_escaped = shell_escape(&remote_dir);

        // 1. Create remote workspace directory
        let mkdir_cmd = format!("mkdir -p {remote_dir_escaped}");
        let (_stdout, stderr, code) = ssh_exec(&self.config, &mkdir_cmd).await?;
        if code != 0 {
            return Err(SynapticError::Tool(format!(
                "ssh: failed to create remote workspace (exit {code}): {stderr}"
            )));
        }

        // 2. Seed workspace via rsync
        let host_dir = req.workspace.host_dir.to_string_lossy().to_string();
        let rsync_args = self.rsync_args(&host_dir, &remote_dir);
        let output = Command::new("rsync")
            .args(&rsync_args)
            .output()
            .await
            .map_err(|e| SynapticError::Tool(format!("rsync failed: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SynapticError::Tool(format!(
                "rsync failed (exit {}): {stderr}",
                output.status.code().unwrap_or(-1)
            )));
        }

        // 3. Run setup command if provided
        if let Some(ref setup) = req.setup_command {
            let setup_escaped = shell_escape(setup);
            let cmd = format!("cd {remote_dir_escaped} && sh -c {setup_escaped}");
            let (_out, stderr, code) = ssh_exec(&self.config, &cmd).await?;
            if code != 0 {
                return Err(SynapticError::Tool(format!(
                    "ssh: setup command failed (exit {code}): {stderr}"
                )));
            }
        }

        // 4. Build the backend
        let backend = Arc::new(SshBackend {
            config: self.config.clone(),
            remote_dir: PathBuf::from(&remote_dir),
        });

        // 5. Wrap in FsBridge
        let container_path = PathBuf::from(&remote_dir);
        let mount = MountMapping {
            host_path: req.workspace.host_dir.clone(),
            container_path: container_path.clone(),
            read_only: matches!(
                req.workspace.access,
                super::types::WorkspaceAccess::ReadOnly
            ),
        };
        let bridge = Arc::new(FsBridge::new(backend, vec![mount], vec![container_path]));

        let now = Utc::now();
        let info = SandboxInstanceInfo {
            runtime_id: scope.clone(),
            provider_id: "ssh".into(),
            runtime_label: format!("ssh:{}", self.config.target),
            scope_key: scope.clone(),
            image: None,
            created_at: now,
            last_used_at: now,
        };

        Ok(SandboxInstance {
            runtime_id: scope.clone(),
            backend: bridge,
            info,
        })
    }

    async fn destroy(&self, runtime_id: &str) -> Result<(), SynapticError> {
        let remote_dir = self
            .config
            .workspace_root
            .join(runtime_id)
            .to_string_lossy()
            .to_string();
        let escaped = shell_escape(&remote_dir);
        let cmd = format!("rm -rf {escaped}");
        let (_stdout, stderr, code) = ssh_exec(&self.config, &cmd).await?;
        if code != 0 {
            return Err(SynapticError::Tool(format!(
                "ssh: destroy failed (exit {code}): {stderr}"
            )));
        }
        Ok(())
    }

    async fn status(&self, runtime_id: &str) -> Result<SandboxStatus, SynapticError> {
        let remote_dir = self
            .config
            .workspace_root
            .join(runtime_id)
            .to_string_lossy()
            .to_string();
        let escaped = shell_escape(&remote_dir);
        let cmd = format!("test -d {escaped} && echo running || echo notfound");
        let (stdout, _stderr, _code) = ssh_exec(&self.config, &cmd).await?;
        match stdout.trim() {
            "running" => Ok(SandboxStatus::Running),
            _ => Ok(SandboxStatus::NotFound),
        }
    }

    async fn list(&self) -> Result<Vec<SandboxInstanceInfo>, SynapticError> {
        let root = self.config.workspace_root.to_string_lossy().to_string();
        let escaped = shell_escape(&root);
        let cmd = format!("test -d {escaped} && ls -1 {escaped} 2>/dev/null || true");
        let (stdout, _stderr, _code) = ssh_exec(&self.config, &cmd).await?;

        let now = Utc::now();
        let infos: Vec<SandboxInstanceInfo> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|name| SandboxInstanceInfo {
                runtime_id: name.to_string(),
                provider_id: "ssh".into(),
                runtime_label: format!("ssh:{}", self.config.target),
                scope_key: name.to_string(),
                image: None,
                created_at: now,
                last_used_at: now,
            })
            .collect();
        Ok(infos)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn shell_escape_with_single_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn shell_escape_with_special_chars() {
        assert_eq!(shell_escape("a; rm -rf /"), "'a; rm -rf /'");
    }

    #[test]
    fn shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn parse_target_with_port() {
        let (host, port) = parse_target("user@example.com:2222");
        assert_eq!(host, "user@example.com");
        assert_eq!(port, Some(2222));
    }

    #[test]
    fn parse_target_without_port() {
        let (host, port) = parse_target("user@example.com");
        assert_eq!(host, "user@example.com");
        assert_eq!(port, None);
    }

    #[test]
    fn parse_target_with_non_numeric_port() {
        // user@host:path should not be parsed as a port
        let (host, port) = parse_target("user@host:/some/path");
        assert_eq!(host, "user@host:/some/path");
        assert_eq!(port, None);
    }

    #[test]
    fn ssh_base_args_minimal() {
        let config = SshProviderConfig {
            target: "user@host".into(),
            identity_file: None,
            strict_host_key_checking: true,
            workspace_root: "/tmp/sandboxes".into(),
            workspace_mode: SshWorkspaceMode::default(),
        };
        let args = ssh_base_args(&config);
        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"StrictHostKeyChecking=yes".to_string()));
        assert!(args.contains(&"BatchMode=yes".to_string()));
        assert!(args.contains(&"user@host".to_string()));
        assert!(!args.contains(&"-p".to_string()));
    }

    #[test]
    fn ssh_base_args_with_port_and_key() {
        let config = SshProviderConfig {
            target: "user@host:2222".into(),
            identity_file: Some("/home/user/.ssh/id_ed25519".into()),
            strict_host_key_checking: false,
            workspace_root: "/tmp/sandboxes".into(),
            workspace_mode: SshWorkspaceMode::Mirror,
        };
        let args = ssh_base_args(&config);
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/user/.ssh/id_ed25519".to_string()));
        assert!(args.contains(&"StrictHostKeyChecking=no".to_string()));
    }

    #[test]
    fn rsync_args_trailing_slash() {
        let config = SshProviderConfig {
            target: "user@host".into(),
            identity_file: None,
            strict_host_key_checking: true,
            workspace_root: "/remote/root".into(),
            workspace_mode: SshWorkspaceMode::default(),
        };
        let provider = SshProvider::new(config);
        let args = provider.rsync_args("/local/dir", "/remote/workspace");
        // Source should have trailing slash
        assert!(args.iter().any(|a| a == "/local/dir/"));
        // Dest should have trailing slash
        assert!(args.iter().any(|a| a == "user@host:/remote/workspace/"));
    }

    #[test]
    fn default_workspace_mode_is_mirror() {
        let mode = SshWorkspaceMode::default();
        assert_eq!(mode, SshWorkspaceMode::Mirror);
    }
}
