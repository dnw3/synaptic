# Callbacks

Synapse provides an event-driven callback system for observing agent execution. The `CallbackHandler` trait receives `RunEvent` values at key lifecycle points -- when a run starts, when the LLM is called, when tools are executed, and when the run finishes or fails.

## The `CallbackHandler` Trait

The trait is defined in `synaptic_core`:

```rust
#[async_trait]
pub trait CallbackHandler: Send + Sync {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapseError>;
}
```

A single method receives all event types. Handlers are `Send + Sync` so they can be shared across async tasks.

## `RunEvent` Variants

The `RunEvent` enum covers the full agent lifecycle:

| Variant | Fields | When It Fires |
|---------|--------|---------------|
| `RunStarted` | `run_id`, `session_id` | At the beginning of an agent run |
| `RunStep` | `run_id`, `step` | At each iteration of the agent loop |
| `LlmCalled` | `run_id`, `message_count` | When the LLM is invoked with messages |
| `ToolCalled` | `run_id`, `tool_name` | When a tool is executed |
| `RunFinished` | `run_id`, `output` | When the agent produces a final answer |
| `RunFailed` | `run_id`, `error` | When the agent run fails with an error |

`RunEvent` implements `Clone`, so handlers can store copies of events for later inspection.

## Built-in Handlers

Synapse ships with four callback handlers:

| Handler | Purpose |
|---------|---------|
| [RecordingCallback](recording.md) | Records all events in memory for later inspection |
| [TracingCallback](tracing.md) | Emits structured `tracing` spans and events |
| [StdOutCallbackHandler](stdout.md) | Prints events to stdout (with optional verbose mode) |
| [CompositeCallback](composite.md) | Dispatches events to multiple handlers |

## Implementing a Custom Handler

You can implement `CallbackHandler` to add your own observability:

```rust
use async_trait::async_trait;
use synaptic_core::{CallbackHandler, RunEvent, SynapseError};

struct MetricsCallback;

#[async_trait]
impl CallbackHandler for MetricsCallback {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapseError> {
        match event {
            RunEvent::LlmCalled { message_count, .. } => {
                // Record to your metrics system
                println!("LLM called with {message_count} messages");
            }
            RunEvent::ToolCalled { tool_name, .. } => {
                println!("Tool executed: {tool_name}");
            }
            _ => {}
        }
        Ok(())
    }
}
```

## Guides

- [Recording Callback](recording.md) -- capture events in memory for testing and inspection
- [Tracing Callback](tracing.md) -- integrate with the Rust `tracing` ecosystem
- [Composite Callback](composite.md) -- dispatch events to multiple handlers simultaneously
