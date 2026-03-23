# Plugin System

Synaptic's plugin system lets you extend agents with tools, event subscribers, memory providers, services, and interceptors -- all through a unified registration API. Plugins declare their capabilities via a manifest and register components through a scoped `PluginApi`. The registry supports hot-disable for runtime plugin management.

## Setup

Enable the `plugin` feature on the `synaptic-config` crate (or through the facade):

```toml
[dependencies]
synaptic = { version = "0.4", features = ["plugin"] }
```

## Plugin Trait

Every plugin implements the `Plugin` trait. The lifecycle has three stages: **manifest** (declare metadata), **register** (add components), and **start/stop** (runtime hooks).

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
        // Register tools, subscribers, services, etc.
        Ok(())
    }

    async fn start(&self, ctx: PluginContext) -> Result<(), SynapticError> {
        // ctx.data_dir is the plugin-specific data directory
        println!("Plugin data dir: {:?}", ctx.data_dir);
        Ok(())
    }

    async fn stop(&self) -> Result<(), SynapticError> {
        // Cleanup on shutdown
        Ok(())
    }
}
```

The `start` and `stop` methods have default no-op implementations, so you only need to override them if your plugin requires initialization or cleanup.

## PluginManifest

The manifest declares metadata and capabilities:

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
    slot: None, // or Some(PluginSlot::Memory) for slot plugins
};
```

**Capabilities** describe what the plugin provides:

| Variant | Description |
|---------|-------------|
| `Tools` | Registers agent tools |
| `Hooks` | Subscribes to lifecycle events |
| `Channels` | Communication channels |
| `Providers` | Model or embedding providers |
| `HttpRoutes` | HTTP endpoint handlers |
| `Commands` | CLI commands |
| `Services` | Background services |
| `CanvasRenderers` | UI rendering extensions |
| `Memory` | Memory providers |

**Slots** (`PluginSlot::Memory`, `PluginSlot::ContextEngine`) are exclusive -- only one plugin can occupy a given slot at a time.

## PluginApi (Scoped Registration)

During `register()`, your plugin receives a `PluginApi` scoped to its plugin ID. All registrations are automatically tracked by the registry.

```rust
use synaptic::config::plugin::PluginApi;
use synaptic::core::SynapticError;
use std::sync::Arc;
use async_trait::async_trait;

#[async_trait]
impl Plugin for MyPlugin {
    // ... manifest() ...

    async fn register(&self, api: &mut PluginApi<'_>) -> Result<(), SynapticError> {
        // Register a tool
        api.register_tool(Arc::new(MySearchTool));

        // Register an event subscriber with priority (lower = earlier)
        api.register_event_subscriber(Arc::new(MySubscriber), 10);

        // Register a background service
        api.register_service(Box::new(MyBackgroundService));

        // Register a middleware interceptor
        api.register_interceptor(Arc::new(MyInterceptor));

        // Register a memory provider (claims the Memory slot)
        api.register_memory(Arc::new(MyMemoryProvider));

        // Access the plugin's own ID
        println!("Registering as: {}", api.plugin_id());

        Ok(())
    }
}
```

## Service Trait

Long-running background services implement the `Service` trait:

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
        // Start background work (e.g., metrics collection)
        Ok(())
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn stop(&self) {
        // Graceful shutdown
    }
}
```

## PluginRegistry (Hot-Disable)

The `PluginRegistry` manages all registered plugins and their components. It supports hot-disabling plugins at runtime without restarting the agent.

```rust
use synaptic::config::plugin::PluginRegistry;
use synaptic::config::plugin::EventBus;
use std::sync::Arc;

// Create a registry
let event_bus = Arc::new(EventBus::new());
let mut registry = PluginRegistry::new(event_bus);

// Register a plugin
let plugin = MyPlugin;
registry.register_plugin(&plugin).await?;

// Inspect registered components
let tools = registry.tools();
let services = registry.services();
let plugins = registry.plugins();

// Check what a plugin registered
if let Some(regs) = registry.plugin_registrations("my-plugin") {
    println!("Tools: {:?}", regs.tools);
    println!("Services: {:?}", regs.services);
    println!("Interceptors: {:?}", regs.interceptors);
    println!("Subscribers: {:?}", regs.subscribers);
}

// Hot-disable: removes all components registered by a plugin
let removed = registry.unregister_plugin("my-plugin");
println!("Removed {} registrations", removed.len());

// Memory slot management
if let Some(provider) = registry.memory_slot() {
    println!("Memory slot owned by: {:?}", registry.memory_slot_owner());
}
```

The `unregister_plugin` method returns a list of all component names that were removed, making it easy to log or audit plugin lifecycle changes.

## PluginHookInterceptor (EventBus Bridge)

`PluginHookInterceptor` (in `synaptic-middleware`) bridges the middleware pipeline to the plugin `EventBus`. It converts middleware lifecycle events into bus events that plugin subscribers can react to:

| Middleware hook | EventBus event |
|-----------------|----------------|
| `before_model` | `BeforeModelCall` |
| `after_model` | `LlmOutput` |
| `wrap_tool_call` | `BeforeToolCall` / `AfterToolCall` |

This lets plugins observe and react to model calls and tool executions without modifying the core middleware chain.

## AgentPlugins

`AgentPlugins` (in `synaptic-graph`) collects interceptors and wires them into the agent's processing pipeline:

```rust
use synaptic::graph::plugins::AgentPlugins;
use std::sync::Arc;

let plugins = AgentPlugins::new()
    .with_interceptor(Arc::new(MyInterceptor));

// Or build incrementally
let mut plugins = AgentPlugins::new();
plugins.add_interceptor(Arc::new(AnotherInterceptor));

// Get the composed chain for use in agent execution
let chain = plugins.interceptor_chain();
```

## Example Plugin

Here is a complete plugin that registers a custom tool and a background service:

```rust
use synaptic::config::plugin::{
    Plugin, PluginApi, PluginContext, PluginManifest,
    PluginCapability, Service,
};
use synaptic::core::{Tool, ToolDefinition, SynapticError};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// -- Tool ----------------------------------------------------------

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

// -- Service -------------------------------------------------------

struct HealthService;

#[async_trait]
impl Service for HealthService {
    fn id(&self) -> &str { "health" }
    async fn start(&self) -> Result<(), SynapticError> { Ok(()) }
    async fn health_check(&self) -> bool { true }
    async fn stop(&self) {}
}

// -- Plugin --------------------------------------------------------

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
