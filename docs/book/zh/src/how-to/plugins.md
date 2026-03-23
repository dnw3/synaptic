# 插件系统

Synaptic 的插件系统允许你通过工具、事件订阅者、内存提供者、服务和拦截器来扩展 Agent——全部通过统一的注册 API 完成。插件通过清单声明自身能力，并通过作用域化的 `PluginApi` 注册组件。注册中心支持热禁用，方便运行时管理插件。

## 设置

在 `synaptic-config` crate（或通过 facade）中启用 `plugin` 特性：

```toml
[dependencies]
synaptic = { version = "0.4", features = ["plugin"] }
```

## Plugin Trait

每个插件都实现 `Plugin` trait。生命周期分三个阶段：**manifest**（声明元数据）、**register**（注册组件）和 **start/stop**（运行时钩子）。

```rust
use synaptic::config::plugin::{Plugin, PluginContext, PluginApi, PluginManifest};
use synaptic::core::SynapticError;
use async_trait::async_trait;

pub struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "my-plugin".into(),
            version: "0.1.0".into(),
            description: "A custom plugin".into(),
            author: Some("Your Name".into()),
            license: Some("MIT".into()),
            capabilities: vec![],
            slot: None,
        }
    }

    async fn register(&self, api: &mut PluginApi<'_>) -> Result<(), SynapticError> {
        // 注册工具、订阅者、服务等
        Ok(())
    }

    async fn start(&self, ctx: PluginContext) -> Result<(), SynapticError> {
        // ctx.data_dir 是插件专用的数据目录
        println!("Plugin data dir: {:?}", ctx.data_dir);
        Ok(())
    }

    async fn stop(&self) -> Result<(), SynapticError> {
        // 关闭时清理
        Ok(())
    }
}
```

`start` 和 `stop` 方法有默认的空实现，因此只有在插件需要初始化或清理时才需要覆盖。

## PluginManifest

清单声明元数据和能力：

```rust
use synaptic::config::plugin::{PluginManifest, PluginCapability, PluginSlot};

let manifest = PluginManifest {
    name: "search-plugin".into(),
    version: "1.0.0".into(),
    description: "Adds web search tools".into(),
    author: Some("Team".into()),
    license: Some("Apache-2.0".into()),
    capabilities: vec![
        PluginCapability::Tools,
        PluginCapability::Hooks,
    ],
    slot: None, // 或 Some(PluginSlot::Memory) 用于插槽型插件
};
```

**能力（Capabilities）** 描述插件提供的功能：

| 变体 | 说明 |
|------|------|
| `Tools` | 注册 Agent 工具 |
| `Hooks` | 订阅生命周期事件 |
| `Channels` | 通信通道 |
| `Providers` | 模型或嵌入提供者 |
| `HttpRoutes` | HTTP 端点处理器 |
| `Commands` | CLI 命令 |
| `Services` | 后台服务 |
| `CanvasRenderers` | UI 渲染扩展 |
| `Memory` | 内存提供者 |

**插槽（Slots）**（`PluginSlot::Memory`、`PluginSlot::ContextEngine`）是排他性的——同一时间只有一个插件可以占据某个插槽。

## PluginApi（作用域化注册）

在 `register()` 中，插件会收到一个以其插件 ID 为作用域的 `PluginApi`。所有注册都会被注册中心自动跟踪。

```rust
use synaptic::config::plugin::PluginApi;
use synaptic::core::SynapticError;
use std::sync::Arc;
use async_trait::async_trait;

#[async_trait]
impl Plugin for MyPlugin {
    // ... manifest() ...

    async fn register(&self, api: &mut PluginApi<'_>) -> Result<(), SynapticError> {
        // 注册工具
        api.register_tool(Arc::new(MySearchTool));

        // 注册事件订阅者，带优先级（数值越小越先执行）
        api.register_event_subscriber(Arc::new(MySubscriber), 10);

        // 注册后台服务
        api.register_service(Box::new(MyBackgroundService));

        // 注册中间件拦截器
        api.register_interceptor(Arc::new(MyInterceptor));

        // 注册内存提供者（占据 Memory 插槽）
        api.register_memory(Arc::new(MyMemoryProvider));

        // 访问插件自身的 ID
        println!("Registering as: {}", api.plugin_id());

        Ok(())
    }
}
```

## Service Trait

长期运行的后台服务实现 `Service` trait：

```rust
use synaptic::config::plugin::Service;
use synaptic::core::SynapticError;
use async_trait::async_trait;

pub struct MetricsService;

#[async_trait]
impl Service for MetricsService {
    fn id(&self) -> &str {
        "metrics-service"
    }

    async fn start(&self) -> Result<(), SynapticError> {
        // 启动后台工作（如指标采集）
        Ok(())
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn stop(&self) {
        // 优雅关闭
    }
}
```

## PluginRegistry（热禁用）

`PluginRegistry` 管理所有已注册的插件及其组件。它支持在运行时热禁用插件，无需重启 Agent。

```rust
use synaptic::config::plugin::PluginRegistry;
use synaptic::config::plugin::EventBus;
use std::sync::Arc;

// 创建注册中心
let event_bus = Arc::new(EventBus::new());
let mut registry = PluginRegistry::new(event_bus);

// 注册插件
let plugin = MyPlugin;
registry.register_plugin(&plugin).await?;

// 查看已注册的组件
let tools = registry.tools();
let services = registry.services();
let plugins = registry.plugins();

// 检查某个插件注册了哪些组件
if let Some(regs) = registry.plugin_registrations("my-plugin") {
    println!("Tools: {:?}", regs.tools);
    println!("Services: {:?}", regs.services);
    println!("Interceptors: {:?}", regs.interceptors);
    println!("Subscribers: {:?}", regs.subscribers);
}

// 热禁用：移除插件注册的所有组件
let removed = registry.unregister_plugin("my-plugin");
println!("Removed {} registrations", removed.len());

// 内存插槽管理
if let Some(provider) = registry.memory_slot() {
    println!("Memory slot owned by: {:?}", registry.memory_slot_owner());
}
```

`unregister_plugin` 方法返回所有被移除的组件名称列表，便于记录日志或审计插件生命周期变更。

## PluginHookInterceptor（EventBus 桥接）

`PluginHookInterceptor`（位于 `synaptic-middleware`）将中间件管道桥接到插件 `EventBus`。它将中间件生命周期事件转换为总线事件，供插件订阅者响应：

| 中间件钩子 | EventBus 事件 |
|-----------|--------------|
| `before_model` | `BeforeModelCall` |
| `after_model` | `LlmOutput` |
| `wrap_tool_call` | `BeforeToolCall` / `AfterToolCall` |

这使得插件可以在不修改核心中间件链的情况下，观察并响应模型调用和工具执行。

## AgentPlugins

`AgentPlugins`（位于 `synaptic-graph`）收集拦截器并将其接入 Agent 的处理管道：

```rust
use synaptic::graph::plugins::AgentPlugins;
use std::sync::Arc;

let plugins = AgentPlugins::new()
    .with_interceptor(Arc::new(MyInterceptor));

// 或者逐步构建
let mut plugins = AgentPlugins::new();
plugins.add_interceptor(Arc::new(AnotherInterceptor));

// 获取组合后的链，用于 Agent 执行
let chain = plugins.interceptor_chain();
```

## 完整插件示例

下面是一个注册自定义工具和后台服务的完整插件示例：

```rust
use synaptic::config::plugin::{
    Plugin, PluginApi, PluginContext, PluginManifest,
    PluginCapability, Service,
};
use synaptic::core::{Tool, ToolDefinition, SynapticError};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// -- 工具 ----------------------------------------------------------

struct PingTool;

#[async_trait]
impl Tool for PingTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "ping".into(),
            description: "Returns pong".into(),
            parameters: serde_json::json!({}),
        }
    }

    async fn call(&self, _input: Value) -> Result<String, SynapticError> {
        Ok("pong".into())
    }
}

// -- 服务 ----------------------------------------------------------

struct HealthService;

#[async_trait]
impl Service for HealthService {
    fn id(&self) -> &str { "health" }
    async fn start(&self) -> Result<(), SynapticError> { Ok(()) }
    async fn health_check(&self) -> bool { true }
    async fn stop(&self) {}
}

// -- 插件 ----------------------------------------------------------

pub struct PingPlugin;

#[async_trait]
impl Plugin for PingPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "ping-plugin".into(),
            version: "0.1.0".into(),
            description: "Adds a ping tool and health service".into(),
            author: None,
            license: None,
            capabilities: vec![
                PluginCapability::Tools,
                PluginCapability::Services,
            ],
            slot: None,
        }
    }

    async fn register(&self, api: &mut PluginApi<'_>) -> Result<(), SynapticError> {
        api.register_tool(Arc::new(PingTool));
        api.register_service(Box::new(HealthService));
        Ok(())
    }

    async fn start(&self, ctx: PluginContext) -> Result<(), SynapticError> {
        println!("PingPlugin started, data dir: {:?}", ctx.data_dir);
        Ok(())
    }

    async fn stop(&self) -> Result<(), SynapticError> {
        println!("PingPlugin stopped");
        Ok(())
    }
}
```
