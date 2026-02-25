# Prometheus Metrics

Export agent metrics in Prometheus text exposition format. The `synaptic-metrics` crate wraps `MetricsCallback` and serves a `/metrics` HTTP endpoint for scraping.

## Setup

```toml
[dependencies]
synaptic = { version = "0.3", features = ["metrics", "callbacks"] }
```

## Quick Start

```rust,ignore
use std::sync::Arc;
use synaptic::callbacks::MetricsCallback;
use synaptic::metrics::PrometheusExporter;

// Create a MetricsCallback (attach to your agent/model)
let metrics = Arc::new(MetricsCallback::new());

// Create exporter and serve
let exporter = PrometheusExporter::new(metrics.clone());
let handle = exporter.serve("0.0.0.0:9090").await?;

println!("Prometheus metrics at http://{}/metrics", handle.addr());

// ... run your agent ...

// Stop the server when done
handle.stop().await;
```

## Rendered Metrics

The exporter renders these metrics (prefix defaults to `synaptic`):

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `synaptic_model_calls_total` | counter | -- | Total LLM API calls |
| `synaptic_model_latency_seconds` | gauge | -- | Average model call latency |
| `synaptic_model_errors_total` | counter | -- | Total model call errors |
| `synaptic_tokens_input_total` | counter | -- | Total input tokens consumed |
| `synaptic_tokens_output_total` | counter | -- | Total output tokens consumed |
| `synaptic_tool_calls_total` | counter | `tool` | Per-tool call count |
| `synaptic_tool_latency_seconds` | gauge | `tool` | Per-tool average latency |
| `synaptic_tool_errors_total` | counter | `tool` | Per-tool error count |

## Custom Prefix

```rust,ignore
let exporter = PrometheusExporter::new(metrics.clone())
    .with_prefix("myapp");
// Metrics will be named: myapp_model_calls_total, etc.
```

## Rendering Without a Server

If you prefer to integrate with an existing HTTP framework, call `render()` directly:

```rust,ignore
let exporter = PrometheusExporter::new(metrics.clone());
let text = exporter.render().await;
// text is Prometheus text exposition format, e.g.:
// # HELP synaptic_model_calls_total Total model calls
// # TYPE synaptic_model_calls_total counter
// synaptic_model_calls_total 42
```

## Prometheus scrape_config Example

```yaml
scrape_configs:
  - job_name: 'synaptic-agent'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```
