# Tracing Callback

`TracingCallback` integrates Synaptic's callback system with the Rust [`tracing`](https://docs.rs/tracing) ecosystem. Instead of storing events in memory, it emits structured tracing spans and events that flow into whatever subscriber you have configured -- terminal output, JSON logs, OpenTelemetry, etc.

## Setup

First, initialize a tracing subscriber. The simplest option is the `fmt` subscriber from `tracing-subscriber`:

```rust
use tracing_subscriber;

// Initialize the default subscriber (prints to stderr)
tracing_subscriber::fmt::init();
```

Then create the callback:

```rust
use synaptic_callbacks::TracingCallback;

let callback = TracingCallback::new();
```

Pass this callback to your agent or use it with `CompositeCallback`.

## What Gets Logged

`TracingCallback` maps each `RunEvent` variant to a `tracing` call:

| RunEvent | Tracing Level | Key Fields |
|----------|---------------|------------|
| `RunStarted` | `info!` | `run_id`, `session_id` |
| `RunStep` | `info!` | `run_id`, `step` |
| `LlmCalled` | `info!` | `run_id`, `message_count` |
| `ToolCalled` | `info!` | `run_id`, `tool_name` |
| `RunFinished` | `info!` | `run_id`, `output_len` |
| `RunFailed` | `error!` | `run_id`, `error` |

All events except `RunFailed` are logged at the `INFO` level. Failures are logged at `ERROR`.

## Example Output

With the default `fmt` subscriber, you might see:

```
2026-02-17T10:30:00.123Z  INFO synaptic: run started run_id="abc-123" session_id="user-1"
2026-02-17T10:30:00.456Z  INFO synaptic: LLM called run_id="abc-123" message_count=3
2026-02-17T10:30:01.234Z  INFO synaptic: tool called run_id="abc-123" tool_name="calculator"
2026-02-17T10:30:01.567Z  INFO synaptic: run finished run_id="abc-123" output_len=42
```

## Integration with the Tracing Ecosystem

Because `TracingCallback` uses the standard `tracing` macros, it works with any compatible subscriber:

- **`tracing-subscriber`** -- terminal formatting, filtering, layering.
- **`tracing-opentelemetry`** -- export spans to Jaeger, Zipkin, or any OTLP collector.
- **`tracing-appender`** -- write logs to rolling files.
- **JSON output** -- use `tracing_subscriber::fmt().json()` for structured log ingestion.

```rust
// Example: JSON-formatted logs
tracing_subscriber::fmt()
    .json()
    .init();

let callback = TracingCallback::new();
```

## When to Use

Use `TracingCallback` when:

- You want production-grade structured logging with minimal setup.
- You are already using the `tracing` ecosystem in your application.
- You need to export agent telemetry to an observability platform (Datadog, Grafana, etc.).

For test-time event inspection, consider [RecordingCallback](recording.md) instead, which stores events for programmatic access.
