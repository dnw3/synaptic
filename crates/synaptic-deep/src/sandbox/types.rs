use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SandboxWorkspace {
    pub host_dir: PathBuf,
    pub access: WorkspaceAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceAccess {
    None,
    #[serde(rename = "ro")]
    ReadOnly,
    #[serde(rename = "rw")]
    ReadWrite,
}

impl Default for WorkspaceAccess {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    None,
    Bridge,
    Host,
    Custom(String),
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSecurityConfig {
    #[serde(default = "default_cap_drop")]
    pub cap_drop: Vec<String>,
    #[serde(default = "default_true")]
    pub read_only_root: bool,
    #[serde(default)]
    pub network_mode: NetworkMode,
    pub seccomp_profile: Option<String>,
    pub apparmor_profile: Option<String>,
    #[serde(default = "default_blocked_host_paths")]
    pub blocked_host_paths: Vec<String>,
}

impl Default for SandboxSecurityConfig {
    fn default() -> Self {
        Self {
            cap_drop: default_cap_drop(),
            read_only_root: true,
            network_mode: NetworkMode::default(),
            seccomp_profile: None,
            apparmor_profile: None,
            blocked_host_paths: default_blocked_host_paths(),
        }
    }
}

fn default_cap_drop() -> Vec<String> {
    vec!["ALL".to_string()]
}

fn default_true() -> bool {
    true
}

fn default_blocked_host_paths() -> Vec<String> {
    vec![
        "/etc",
        "/private/etc",
        "/proc",
        "/sys",
        "/dev",
        "/root",
        "/boot",
        "/run",
        "/var/run",
        "/private/var/run",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxResourceLimits {
    pub memory: Option<String>,
    pub memory_swap: Option<String>,
    pub cpus: Option<f64>,
    pub pids_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    #[serde(default)]
    pub read_only: bool,
}
