# Composite Callback

`CompositeCallback` dispatches each `RunEvent` to multiple callback handlers. This lets you combine different observability strategies without choosing just one -- for example, recording events in memory for tests while also logging them via `tracing`.

## Usage

```rust
use synaptic::callbacks::{CompositeCallback, RecordingCallback, TracingCallback};
use std::sync::Arc;

let recording = Arc::new(RecordingCallback::new());
let tracing_cb = Arc::new(TracingCallback::new());

let composite = CompositeCallback::new(vec![
    recording.clone(),
    tracing_cb,
]);
```

When `composite.on_event(event)` is called, the event is forwarded to each handler in order. If any handler returns an error, the composite stops and propagates that error.

## How It Works

`CompositeCallback` holds a `Vec<Arc<dyn CallbackHandler>>`. On each event:

1. The event is cloned for each handler (since `RunEvent` implements `Clone`).
2. Each handler's `on_event()` is awaited sequentially.
3. If all handlers succeed, `Ok(())` is returned.

```rust
// Pseudocode of the dispatch logic
async fn on_event(&self, event: RunEvent) -> Result<(), SynapticError> {
    for handler in &self.handlers {
        handler.on_event(event.clone()).await?;
    }
    Ok(())
}
```

## Example: Recording + Tracing + Custom

You can mix built-in and custom handlers:

```rust
use async_trait::async_trait;
use synaptic::core::{CallbackHandler, RunEvent, SynapticError};
use synaptic::callbacks::{
    CompositeCallback, RecordingCallback, TracingCallback, StdOutCallbackHandler,
};
use std::sync::Arc;

struct ToolCounter {
    count: Arc<tokio::sync::RwLock<usize>>,
}

#[async_trait]
impl CallbackHandler for ToolCounter {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapticError> {
        if matches!(event, RunEvent::ToolCalled { .. }) {
            *self.count.write().await += 1;
        }
        Ok(())
    }
}

let counter = Arc::new(ToolCounter {
    count: Arc::new(tokio::sync::RwLock::new(0)),
});

let composite = CompositeCallback::new(vec![
    Arc::new(RecordingCallback::new()),
    Arc::new(TracingCallback::new()),
    Arc::new(StdOutCallbackHandler::new()),
    counter.clone(),
]);
```

## When to Use

Use `CompositeCallback` whenever you need more than one callback handler active at the same time. Common combinations:

- **Development**: `StdOutCallbackHandler` + `RecordingCallback` -- see events in the terminal and inspect them programmatically.
- **Testing**: `RecordingCallback` alone is usually sufficient.
- **Production**: `TracingCallback` + custom metrics handler -- structured logs plus application-specific telemetry.
