# 容器沙箱

在容器中安全运行不受信任的代码，支持资源限制。`synaptic-sandbox` crate 提供 `SandboxBackend` trait，支持 Docker 和 Apple Container 后端。

## 安装

```toml
[dependencies]
synaptic = { version = "0.4", features = ["sandbox"] }
```

## SandboxBackend Trait

所有沙箱后端都实现此 trait：

```rust,ignore
use synaptic::sandbox::{SandboxBackend, SandboxResult, ResourceLimits};

let result: SandboxResult = backend.execute("print('hello')", "python", &limits).await?;
println!("标准输出: {}", result.stdout);
println!("标准错误: {}", result.stderr);
println!("退出码: {}", result.exit_code);
```

## Docker 沙箱

默认后端通过 `docker` CLI 执行。需要安装 Docker 并确保可访问。

```rust,ignore
use synaptic::sandbox::{DockerSandbox, SandboxBackend, ResourceLimits};

let sandbox = DockerSandbox::default();

// 检查 Docker 是否可用
if sandbox.is_available().await {
    let limits = ResourceLimits {
        memory_mb: 256,
        cpu_count: 1.0,
        timeout_secs: 30,
        network: false, // 禁用网络访问
    };

    let result = sandbox.execute("print('Hello from Python!')", "python", &limits).await?;
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("Hello from Python!"));
}
```

### 自定义 Docker 镜像

```rust,ignore
let sandbox = DockerSandbox::default()
    .with_image("python", "python:3.12-slim")
    .with_image("javascript", "node:20-alpine")
    .with_docker_host("unix:///var/run/docker.sock");
```

### 默认语言镜像

| 语言 | 默认镜像 |
|------|---------|
| `python` | `python:3-slim` |
| `javascript` | `node:20-slim` |
| `bash` | `bash:latest` |

### 资源限制

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `memory_mb` | `u64` | `256` | 内存限制（兆字节） |
| `cpu_count` | `f32` | `1.0` | CPU 核数 |
| `timeout_secs` | `u64` | `30` | 执行超时（秒） |
| `network` | `bool` | `false` | 是否允许网络访问 |

## Apple Container 沙箱

在 macOS 26+ 上，可以使用 Apple 原生容器运行时。在 `synaptic-sandbox` 上启用 `apple-container` feature：

```rust,ignore
#[cfg(target_os = "macos")]
use synaptic::sandbox::AppleContainerSandbox;

let sandbox = AppleContainerSandbox::default();
if sandbox.is_available().await {
    let result = sandbox.execute("echo hello", "bash", &ResourceLimits::default()).await?;
}
```

此后端需要 `container` CLI 工具（macOS 26+）。如果未找到该工具，`is_available()` 会优雅地返回 `false`。

## SandboxTool（Agent 集成）

`SandboxTool` 将任意 `SandboxBackend` 包装为 `Tool`，可直接用于 Agent：

```rust,ignore
use std::sync::Arc;
use synaptic::sandbox::{DockerSandbox, SandboxTool};

let backend = DockerSandbox::default();
let tool = SandboxTool::new(Arc::new(backend));

// 注册到 agent 工具列表
// 工具接受 JSON 输入: {"code": "...", "language": "python"}
```
