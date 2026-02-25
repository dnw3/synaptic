# Container Sandbox

Run untrusted code safely inside containers with resource limits. The `synaptic-sandbox` crate provides a `SandboxBackend` trait with Docker and Apple Container backends.

## Setup

```toml
[dependencies]
synaptic = { version = "0.3", features = ["sandbox"] }
```

## SandboxBackend Trait

All sandbox backends implement this trait:

```rust,ignore
use synaptic::sandbox::{SandboxBackend, SandboxResult, ResourceLimits};

let result: SandboxResult = backend.execute("print('hello')", "python", &limits).await?;
println!("stdout: {}", result.stdout);
println!("stderr: {}", result.stderr);
println!("exit code: {}", result.exit_code);
```

## Docker Sandbox

The default backend shells out to the `docker` CLI. Requires Docker to be installed and accessible.

```rust,ignore
use synaptic::sandbox::{DockerSandbox, SandboxBackend, ResourceLimits};

let sandbox = DockerSandbox::default();

// Check if Docker is available
if sandbox.is_available().await {
    let limits = ResourceLimits {
        memory_mb: 256,
        cpu_count: 1.0,
        timeout_secs: 30,
        network: false, // disable network access
    };

    let result = sandbox.execute("print('Hello from Python!')", "python", &limits).await?;
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("Hello from Python!"));
}
```

### Custom Docker Images

```rust,ignore
let sandbox = DockerSandbox::default()
    .with_image("python", "python:3.12-slim")
    .with_image("javascript", "node:20-alpine")
    .with_docker_host("unix:///var/run/docker.sock");
```

### Default Language Images

| Language | Default Image |
|----------|--------------|
| `python` | `python:3-slim` |
| `javascript` | `node:20-slim` |
| `bash` | `bash:latest` |

### Resource Limits

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `memory_mb` | `u64` | `256` | Memory limit in megabytes |
| `cpu_count` | `f32` | `1.0` | Number of CPU cores |
| `timeout_secs` | `u64` | `30` | Execution timeout in seconds |
| `network` | `bool` | `false` | Allow network access |

## Apple Container Sandbox

On macOS 26+, use Apple's native container runtime. Enable the `apple-container` feature on `synaptic-sandbox`:

```rust,ignore
#[cfg(target_os = "macos")]
use synaptic::sandbox::AppleContainerSandbox;

let sandbox = AppleContainerSandbox::default();
if sandbox.is_available().await {
    let result = sandbox.execute("echo hello", "bash", &ResourceLimits::default()).await?;
}
```

This backend requires the `container` CLI tool (macOS 26+). It gracefully returns `false` from `is_available()` if the tool is not found.

## SandboxTool (Agent Integration)

`SandboxTool` wraps any `SandboxBackend` as a `Tool` for use with agents:

```rust,ignore
use std::sync::Arc;
use synaptic::sandbox::{DockerSandbox, SandboxTool};

let backend = DockerSandbox::default();
let tool = SandboxTool::new(Arc::new(backend));

// Register with agent tools
// The tool accepts JSON: {"code": "...", "language": "python"}
```
