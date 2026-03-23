# 容器沙箱

在隔离环境中运行 Agent 工作负载，并提供细粒度的安全控制。沙箱系统位于 `synaptic-deep` 中，提供可插拔的 Provider（Docker、SSH），通过包装 `Backend` trait 实现进程级隔离。

## 安装

基础沙箱类型需要 `sandbox` feature。根据需要添加 Provider 对应的 feature：

```toml
[dependencies]
# 仅基础类型
synaptic = { version = "0.4", features = ["sandbox"] }

# 使用 Docker Provider
synaptic = { version = "0.4", features = ["sandbox-docker"] }

# 使用 SSH Provider
synaptic = { version = "0.4", features = ["sandbox-ssh"] }

# 同时使用两个 Provider
synaptic = { version = "0.4", features = ["sandbox-docker", "sandbox-ssh"] }
```

## SandboxProvider Trait

所有沙箱后端都实现 `SandboxProvider` trait。Provider 管理沙箱实例的完整生命周期：创建、状态检查、列表查询和销毁。

```rust,ignore
use synaptic::deep::sandbox::{
    SandboxProvider, SandboxCreateRequest, SandboxInstance,
    SandboxStatus, SandboxInstanceInfo,
};

// 创建沙箱实例
let instance: SandboxInstance = provider.create(request).await?;
println!("运行时 ID: {}", instance.runtime_id);

// 检查状态
let status: SandboxStatus = provider.status(&instance.runtime_id).await?;
// SandboxStatus 变体: Running, Stopped, NotFound

// 列出该 Provider 管理的所有实例
let instances: Vec<SandboxInstanceInfo> = provider.list().await?;

// 使用完毕后销毁
provider.destroy(&instance.runtime_id).await?;
```

`SandboxCreateRequest` 用于配置沙箱：

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

| 变体 | 说明 |
|------|------|
| `None` | 不挂载工作区 |
| `ReadOnly` | 以只读方式挂载工作区 |
| `ReadWrite` | 以读写方式挂载工作区 |

### SandboxResourceLimits

所有字段均为 `Option` -- 省略则使用 Provider 默认值。

| 字段 | 类型 | 说明 |
|------|------|------|
| `memory` | `Option<String>` | 内存限制（例如 `"512m"`） |
| `memory_swap` | `Option<String>` | Swap 限制 |
| `cpus` | `Option<f64>` | CPU 配额 |
| `pids_limit` | `Option<i64>` | 最大进程数 |

## Docker Provider

`DockerProvider` 使用 Docker CLI 创建沙箱容器。每个沙箱实例作为长期运行的容器存在，命令通过 `docker exec` 在容器内执行。需要安装 Docker。

通过 `sandbox-docker` feature 启用。

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

// instance.backend 是 Arc<dyn Backend>，内部使用 `docker exec`
// 并通过 FsBridge 包装实现路径转换
```

### DockerProviderConfig

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `image` | `"synapse-sandbox:bookworm-slim"` | 容器镜像 |
| `container_prefix` | `"synapse-sbx-"` | 容器名称前缀 |
| `tmpfs_mounts` | `["/tmp", "/var/tmp", "/run"]` | tmpfs 挂载点 |
| `user` | `None` | 运行用户/组（例如 `"1000:1000"`） |

## SSH Provider

`SshProvider` 通过 SSH 在远程主机上执行命令。适用于在专用构建服务器或虚拟机上运行沙箱工作负载，无需 Docker。

通过 `sandbox-ssh` feature 启用。

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

| 模式 | 说明 |
|------|------|
| `Mirror` | 本地工作区为权威源，同步到远程（默认） |
| `Remote` | 远程工作区为权威源，本地路径映射到远程 |

### SshProviderConfig

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `target` | -- | SSH 目标，格式为 `user@host:port` |
| `identity_file` | `None` | SSH 私钥路径 |
| `strict_host_key_checking` | `true` | 拒绝未知主机密钥 |
| `workspace_root` | -- | 远程沙箱工作区根目录 |
| `workspace_mode` | `Mirror` | 工作区文件同步方式 |

## FsBridge

`FsBridge` 是一个 `Backend` 装饰器，负责在宿主机和容器之间转换文件路径。它还实施路径安全策略：拒绝路径穿越（`..`）尝试以及对只读挂载的写入操作。

`DockerProvider` 会自动为其后端包装 `FsBridge`。你也可以直接使用：

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

`allowed_roots` 参数限制可访问的宿主机路径。任何超出这些根路径的访问都会被拒绝。

## Security

`SandboxSecurityConfig` 提供纵深防御的默认配置。`validate_sandbox_security` 函数在创建沙箱前检查配置及其挂载。

```rust,ignore
use synaptic::deep::sandbox::{
    SandboxSecurityConfig, NetworkMode, BindMount,
    validate_sandbox_security,
};

let security = SandboxSecurityConfig::default();
// 默认值:
//   cap_drop: ["ALL"]
//   read_only_root: true
//   network_mode: NetworkMode::None
//   blocked_host_paths: ["/etc", "/private/etc", "/proc", "/sys",
//                        "/dev", "/root", "/boot", "/run",
//                        "/var/run", "/private/var/run"]

// 创建沙箱前进行验证
validate_sandbox_security(&security, &mounts)?;
```

### 验证规则

`validate_sandbox_security` 函数执行以下检查：

- **禁止宿主机网络** -- `NetworkMode::Host` 会被拒绝。
- **敏感宿主路径拦截** -- 挂载到敏感目录（如 `/etc`、`/proc`、`/sys`）的请求会被拒绝。
- **保留容器目标路径** -- 某些容器端路径不允许被挂载覆盖。
- **禁止无限制安全配置** -- Seccomp 和 AppArmor 配置设为 `"unconfined"` 时会被拒绝。
- **要求绝对路径** -- 所有挂载路径必须是绝对路径。

### NetworkMode

| 变体 | 说明 |
|------|------|
| `None` | 无网络访问（默认） |
| `Bridge` | 隔离桥接网络 |
| `Host` | 宿主机网络（验证时会被拒绝） |
| `Custom(String)` | 指定 Docker 网络名称 |

### SandboxSecurityConfig 字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `cap_drop` | `Vec<String>` | `["ALL"]` | 要移除的 Linux capabilities |
| `read_only_root` | `bool` | `true` | 以只读方式挂载根文件系统 |
| `network_mode` | `NetworkMode` | `None` | 容器网络模式 |
| `seccomp_profile` | `Option<String>` | `None` | 自定义 seccomp 配置路径 |
| `apparmor_profile` | `Option<String>` | `None` | 自定义 AppArmor 配置 |
| `blocked_host_paths` | `Vec<PathBuf>` | （见上文） | 禁止挂载的宿主机路径 |

## Provider Registry

使用 `SandboxProviderRegistry` 管理多个 Provider，并在运行时按需选择：

```rust,ignore
use std::sync::Arc;
use synaptic::deep::sandbox::SandboxProviderRegistry;

let mut registry = SandboxProviderRegistry::new();

// 注册 Provider
registry.register(Arc::new(docker_provider));
registry.register(Arc::new(ssh_provider));

// 列出可用的 Provider ID
let ids: Vec<String> = registry.list_ids();

// 按 ID 查找 Provider
if let Some(provider) = registry.get("docker") {
    let instance = provider.create(request).await?;
}
```
