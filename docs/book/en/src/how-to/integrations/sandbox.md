# Container Sandbox

Run agent workloads in isolated environments with fine-grained security controls. The sandbox system lives in `synaptic-deep` and provides pluggable providers (Docker, SSH) that wrap the `Backend` trait with process-level isolation.

## Setup

Base sandbox types require the `sandbox` feature. Add provider-specific features as needed:

```toml
[dependencies]
# Base types only
synaptic = { version = "0.4", features = ["sandbox"] }

# With Docker provider
synaptic = { version = "0.4", features = ["sandbox-docker"] }

# With SSH provider
synaptic = { version = "0.4", features = ["sandbox-ssh"] }

# Both providers
synaptic = { version = "0.4", features = ["sandbox-docker", "sandbox-ssh"] }
```

## SandboxProvider Trait

All sandbox backends implement the `SandboxProvider` trait. A provider manages the full lifecycle of sandbox instances: creation, status checks, listing, and destruction.

```rust,ignore
use synaptic::deep::sandbox::{
    SandboxProvider, SandboxCreateRequest, SandboxInstance,
    SandboxStatus, SandboxInstanceInfo,
};

// Create a sandbox instance
let instance: SandboxInstance = provider.create(request).await?;
println!("Runtime ID: {}", instance.runtime_id);

// Check status
let status: SandboxStatus = provider.status(&instance.runtime_id).await?;
// SandboxStatus variants: Running, Stopped, NotFound

// List all instances managed by this provider
let instances: Vec<SandboxInstanceInfo> = provider.list().await?;

// Destroy when done
provider.destroy(&instance.runtime_id).await?;
```

A `SandboxCreateRequest` configures the sandbox:

```rust,ignore
use std::collections::HashMap;
use std::path::PathBuf;
use synaptic::deep::sandbox::{
    SandboxCreateRequest, SandboxWorkspace, WorkspaceAccess,
    SandboxSecurityConfig, SandboxResourceLimits, BindMount,
};

let request = SandboxCreateRequest {
    scope_key: "my-agent-session".into(),
    workspace: SandboxWorkspace {
        host_dir: PathBuf::from("/tmp/agent-workspace"),
        access: WorkspaceAccess::ReadWrite,
    },
    security: SandboxSecurityConfig::default(),
    resources: SandboxResourceLimits::default(),
    extra_mounts: vec![
        BindMount {
            host_path: PathBuf::from("/data/models"),
            container_path: PathBuf::from("/mnt/models"),
            read_only: true,
        },
    ],
    setup_command: Some("pip install numpy".into()),
    env: HashMap::from([("PYTHONPATH".into(), "/app".into())]),
};
```

### WorkspaceAccess

| Variant | Description |
|---------|-------------|
| `None` | No workspace mounted |
| `ReadOnly` | Workspace mounted read-only |
| `ReadWrite` | Workspace mounted read-write |

### SandboxResourceLimits

All fields are `Option` -- omit to use provider defaults.

| Field | Type | Description |
|-------|------|-------------|
| `memory` | `Option<String>` | Memory limit (e.g. `"512m"`) |
| `memory_swap` | `Option<String>` | Swap limit |
| `cpus` | `Option<f64>` | CPU quota |
| `pids_limit` | `Option<i64>` | Max number of processes |

## Docker Provider

The `DockerProvider` creates sandboxed containers using the Docker CLI. Each sandbox instance runs as a long-lived container; commands are executed inside it via `docker exec`. Requires Docker to be installed.

Enable with the `sandbox-docker` feature.

```rust,ignore
use std::sync::Arc;
use synaptic::deep::sandbox::{
    DockerProvider, DockerProviderConfig,
    SandboxProvider, SandboxCreateRequest,
};

let config = DockerProviderConfig {
    image: "synapse-sandbox:bookworm-slim".into(),
    container_prefix: "synapse-sbx-".into(),
    tmpfs_mounts: vec!["/tmp".into(), "/var/tmp".into(), "/run".into()],
    user: Some("1000:1000".into()),
};

let provider = DockerProvider::new(config);
let instance = provider.create(request).await?;

// The instance.backend is an Arc<dyn Backend> that uses `docker exec`
// internally, wrapped with FsBridge for path translation
```

### DockerProviderConfig

| Field | Default | Description |
|-------|---------|-------------|
| `image` | `"synapse-sandbox:bookworm-slim"` | Container image |
| `container_prefix` | `"synapse-sbx-"` | Prefix for container names |
| `tmpfs_mounts` | `["/tmp", "/var/tmp", "/run"]` | Tmpfs mount points |
| `user` | `None` | User/group to run as (e.g. `"1000:1000"`) |

## SSH Provider

The `SshProvider` executes commands on a remote host over SSH. Useful for running sandboxed workloads on dedicated build servers or VMs without Docker.

Enable with the `sandbox-ssh` feature.

```rust,ignore
use std::path::PathBuf;
use std::sync::Arc;
use synaptic::deep::sandbox::{
    SshProvider, SshProviderConfig, SshWorkspaceMode,
    SandboxProvider,
};

let config = SshProviderConfig {
    target: "agent@build-server:22".into(),
    identity_file: Some(PathBuf::from("/home/user/.ssh/id_ed25519")),
    strict_host_key_checking: true,
    workspace_root: PathBuf::from("/var/sandboxes"),
    workspace_mode: SshWorkspaceMode::Mirror,
};

let provider = SshProvider::new(config);
let instance = provider.create(request).await?;
```

### SshWorkspaceMode

| Mode | Description |
|------|-------------|
| `Mirror` | Local workspace is canonical; synced to remote (default) |
| `Remote` | Remote workspace is canonical; local path maps to remote |

### SshProviderConfig

| Field | Default | Description |
|-------|---------|-------------|
| `target` | -- | SSH target in `user@host:port` format |
| `identity_file` | `None` | Path to SSH private key |
| `strict_host_key_checking` | `true` | Reject unknown host keys |
| `workspace_root` | -- | Base directory for sandboxed workspaces on remote |
| `workspace_mode` | `Mirror` | How workspace files are synchronized |

## FsBridge

`FsBridge` is a `Backend` decorator that translates file paths between the host and the container. It also enforces path security: rejecting traversal attempts (`..`) and writes to read-only mounts.

The `DockerProvider` automatically wraps its backend with `FsBridge`. You can also use it directly:

```rust,ignore
use std::sync::Arc;
use std::path::PathBuf;
use synaptic::deep::sandbox::{FsBridge, MountMapping};

let bridge = FsBridge::new(
    inner_backend,
    vec![
        MountMapping {
            host_path: PathBuf::from("/tmp/workspace"),
            container_path: PathBuf::from("/workspace"),
            read_only: false,
        },
        MountMapping {
            host_path: PathBuf::from("/data/readonly"),
            container_path: PathBuf::from("/mnt/data"),
            read_only: true,
        },
    ],
    vec![PathBuf::from("/tmp/workspace"), PathBuf::from("/data/readonly")],
);
```

The `allowed_roots` parameter restricts which host paths can be accessed. Any path outside these roots is rejected.

## Security

The `SandboxSecurityConfig` provides defense-in-depth defaults. The `validate_sandbox_security` function checks a configuration and its mounts before sandbox creation.

```rust,ignore
use synaptic::deep::sandbox::{
    SandboxSecurityConfig, NetworkMode, BindMount,
    validate_sandbox_security,
};

let security = SandboxSecurityConfig::default();
// Defaults:
//   cap_drop: ["ALL"]
//   read_only_root: true
//   network_mode: NetworkMode::None
//   blocked_host_paths: ["/etc", "/private/etc", "/proc", "/sys",
//                        "/dev", "/root", "/boot", "/run",
//                        "/var/run", "/private/var/run"]

// Validate before creating sandbox
validate_sandbox_security(&security, &mounts)?;
```

### Validation Rules

The `validate_sandbox_security` function enforces:

- **Host networking forbidden** -- `NetworkMode::Host` is rejected.
- **Blocked host paths** -- Mounts targeting sensitive directories (e.g. `/etc`, `/proc`, `/sys`) are rejected.
- **Reserved container targets** -- Certain container-side paths cannot be mounted over.
- **Unconfined profiles forbidden** -- Seccomp and AppArmor profiles set to `"unconfined"` are rejected.
- **Absolute paths required** -- All mount paths must be absolute.

### NetworkMode

| Variant | Description |
|---------|-------------|
| `None` | No network access (default) |
| `Bridge` | Isolated bridge network |
| `Host` | Host network (rejected by validation) |
| `Custom(String)` | Named Docker network |

### SandboxSecurityConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cap_drop` | `Vec<String>` | `["ALL"]` | Linux capabilities to drop |
| `read_only_root` | `bool` | `true` | Mount root filesystem as read-only |
| `network_mode` | `NetworkMode` | `None` | Container network mode |
| `seccomp_profile` | `Option<String>` | `None` | Custom seccomp profile path |
| `apparmor_profile` | `Option<String>` | `None` | Custom AppArmor profile |
| `blocked_host_paths` | `Vec<PathBuf>` | (see above) | Host paths that cannot be mounted |

## Provider Registry

Use `SandboxProviderRegistry` to manage multiple providers and select between them at runtime:

```rust,ignore
use std::sync::Arc;
use synaptic::deep::sandbox::SandboxProviderRegistry;

let mut registry = SandboxProviderRegistry::new();

// Register providers
registry.register(Arc::new(docker_provider));
registry.register(Arc::new(ssh_provider));

// List available provider IDs
let ids: Vec<String> = registry.list_ids();

// Look up a provider by ID
if let Some(provider) = registry.get("docker") {
    let instance = provider.create(request).await?;
}
```
