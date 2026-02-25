//! Prometheus metrics exporter for Synaptic agent telemetry.
//!
//! Wraps [`MetricsCallback`] from `synaptic-callbacks` and exposes collected
//! metrics via an HTTP endpoint in Prometheus text exposition format.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use synaptic_callbacks::MetricsCallback;
//! use synaptic_metrics::PrometheusExporter;
//!
//! # async fn example() {
//! let metrics = Arc::new(MetricsCallback::new());
//! let exporter = PrometheusExporter::new(metrics);
//! let handle = exporter.serve("127.0.0.1:9090").await.unwrap();
//! // ... later ...
//! handle.stop();
//! # }
//! ```

use std::sync::Arc;

use synaptic_callbacks::{MetricsCallback, MetricsSnapshot, ToolMetrics};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Exports [`MetricsCallback`] data as Prometheus text exposition format.
pub struct PrometheusExporter {
    metrics: Arc<MetricsCallback>,
    prefix: String,
}

impl PrometheusExporter {
    /// Create a new exporter wrapping the given metrics callback.
    ///
    /// Uses the default prefix `"synaptic"`.
    pub fn new(metrics: Arc<MetricsCallback>) -> Self {
        Self {
            metrics,
            prefix: "synaptic".to_string(),
        }
    }

    /// Create a new exporter with a custom metric name prefix.
    pub fn with_prefix(metrics: Arc<MetricsCallback>, prefix: impl Into<String>) -> Self {
        Self {
            metrics,
            prefix: prefix.into(),
        }
    }

    /// Render the current metrics as a Prometheus text exposition string.
    pub async fn render(&self) -> String {
        let snap = self.metrics.snapshot().await;
        render_prometheus(&self.prefix, &snap)
    }

    /// Start an HTTP server that responds to `GET /metrics` with Prometheus
    /// text exposition format.
    ///
    /// Binds to the given address (e.g. `"127.0.0.1:9090"` or `"0.0.0.0:0"`
    /// for an OS-assigned port).
    ///
    /// Returns a [`MetricsServerHandle`] that can be used to query the bound
    /// address and stop the server.
    pub async fn serve(
        &self,
        addr: &str,
    ) -> Result<MetricsServerHandle, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let metrics = Arc::clone(&self.metrics);
        let prefix = self.prefix.clone();

        let join_handle = tokio::spawn(async move {
            run_server(listener, metrics, prefix, shutdown_rx).await;
        });

        Ok(MetricsServerHandle {
            addr: local_addr,
            shutdown_tx: Some(shutdown_tx),
            join_handle: Some(join_handle),
        })
    }
}

/// Handle to a running metrics HTTP server.
///
/// Dropping the handle without calling [`stop()`](MetricsServerHandle::stop)
/// will signal shutdown but not wait for the server task to complete.
pub struct MetricsServerHandle {
    addr: std::net::SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MetricsServerHandle {
    /// The address the server is listening on.
    ///
    /// Useful when binding to port `0` to discover the assigned port.
    pub fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    /// Signal the server to shut down.
    pub fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Drop join_handle; the task will finish on its own.
    }

    /// Signal the server to shut down and wait for it to finish.
    pub async fn stop_and_wait(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for MetricsServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn run_server(
    listener: TcpListener,
    metrics: Arc<MetricsCallback>,
    prefix: String,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut stream, _peer)) => {
                        let snap = metrics.snapshot().await;
                        let body = render_prometheus(&prefix, &snap);
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                             Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\
                             \r\n\
                             {}",
                            body.len(),
                            body,
                        );
                        // Best-effort write; ignore errors from misbehaving clients.
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.shutdown().await;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "metrics server accept error");
                    }
                }
            }
            _ = &mut shutdown_rx => {
                tracing::debug!("metrics server shutting down");
                break;
            }
        }
    }
}

/// Render a [`MetricsSnapshot`] as Prometheus text exposition format.
fn render_prometheus(prefix: &str, snap: &MetricsSnapshot) -> String {
    let mut out = String::with_capacity(1024);

    // -- model_calls_total --
    write_metric(
        &mut out,
        prefix,
        "model_calls_total",
        "counter",
        "Total number of model (LLM) calls.",
        &format_u64(snap.total_model_calls),
        None,
    );

    // -- model_latency_seconds --
    let latency_secs = snap.avg_model_latency_ms / 1000.0;
    write_metric(
        &mut out,
        prefix,
        "model_latency_seconds",
        "gauge",
        "Average model call latency in seconds.",
        &format_f64(latency_secs),
        None,
    );

    // -- model_errors_total --
    write_metric(
        &mut out,
        prefix,
        "model_errors_total",
        "counter",
        "Total number of model errors.",
        &format_u64(snap.total_errors),
        None,
    );

    // -- tokens_input_total --
    write_metric(
        &mut out,
        prefix,
        "tokens_input_total",
        "counter",
        "Total input tokens consumed.",
        &format_u64(snap.total_input_tokens),
        None,
    );

    // -- tokens_output_total --
    write_metric(
        &mut out,
        prefix,
        "tokens_output_total",
        "counter",
        "Total output tokens produced.",
        &format_u64(snap.total_output_tokens),
        None,
    );

    // -- per-tool metrics --
    write_per_tool_metrics(&mut out, prefix, &snap.per_tool);

    out
}

fn write_metric(
    out: &mut String,
    prefix: &str,
    name: &str,
    metric_type: &str,
    help: &str,
    value: &str,
    labels: Option<&str>,
) {
    let fqn = format!("{}_{}", prefix, name);
    out.push_str(&format!("# HELP {} {}\n", fqn, help));
    out.push_str(&format!("# TYPE {} {}\n", fqn, metric_type));
    match labels {
        Some(l) => out.push_str(&format!("{}{} {}\n", fqn, l, value)),
        None => out.push_str(&format!("{} {}\n", fqn, value)),
    }
}

fn write_per_tool_metrics(
    out: &mut String,
    prefix: &str,
    per_tool: &std::collections::HashMap<String, ToolMetrics>,
) {
    // Sort tool names for deterministic output.
    let mut tool_names: Vec<&String> = per_tool.keys().collect();
    tool_names.sort();

    // -- tool_calls_total --
    {
        let fqn = format!("{}_tool_calls_total", prefix);
        out.push_str(&format!("# HELP {} Total tool calls per tool.\n", fqn));
        out.push_str(&format!("# TYPE {} counter\n", fqn));
        for name in &tool_names {
            let m = &per_tool[*name];
            out.push_str(&format!(
                "{}{{tool=\"{}\"}} {}\n",
                fqn,
                escape_label_value(name),
                format_u64(m.calls),
            ));
        }
    }

    // -- tool_errors_total --
    {
        let fqn = format!("{}_tool_errors_total", prefix);
        out.push_str(&format!("# HELP {} Total tool errors per tool.\n", fqn));
        out.push_str(&format!("# TYPE {} counter\n", fqn));
        for name in &tool_names {
            let m = &per_tool[*name];
            out.push_str(&format!(
                "{}{{tool=\"{}\"}} {}\n",
                fqn,
                escape_label_value(name),
                format_u64(m.errors),
            ));
        }
    }

    // -- tool_latency_seconds --
    {
        let fqn = format!("{}_tool_latency_seconds", prefix);
        out.push_str(&format!(
            "# HELP {} Average tool call latency in seconds per tool.\n",
            fqn
        ));
        out.push_str(&format!("# TYPE {} gauge\n", fqn));
        for name in &tool_names {
            let m = &per_tool[*name];
            let avg_secs = if m.calls > 0 {
                (m.total_latency_ms as f64 / m.calls as f64) / 1000.0
            } else {
                0.0
            };
            out.push_str(&format!(
                "{}{{tool=\"{}\"}} {}\n",
                fqn,
                escape_label_value(name),
                format_f64(avg_secs),
            ));
        }
    }
}

/// Escape a Prometheus label value (backslash, double-quote, newline).
fn escape_label_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn format_u64(v: u64) -> String {
    v.to_string()
}

fn format_f64(v: f64) -> String {
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{:.6}", v)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn render_empty_snapshot() {
        let snap = MetricsSnapshot {
            total_model_calls: 0,
            total_tool_calls: 0,
            total_errors: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            avg_model_latency_ms: 0.0,
            per_tool: HashMap::new(),
        };
        let output = render_prometheus("synaptic", &snap);

        assert!(output.contains("# HELP synaptic_model_calls_total"));
        assert!(output.contains("# TYPE synaptic_model_calls_total counter"));
        assert!(output.contains("synaptic_model_calls_total 0"));

        assert!(output.contains("synaptic_model_latency_seconds 0"));
        assert!(output.contains("synaptic_model_errors_total 0"));
        assert!(output.contains("synaptic_tokens_input_total 0"));
        assert!(output.contains("synaptic_tokens_output_total 0"));

        // No tool lines (no tool data).
        assert!(!output.contains("tool=\""));
    }

    #[test]
    fn render_with_tool_metrics() {
        let mut per_tool = HashMap::new();
        per_tool.insert(
            "read_file".to_string(),
            ToolMetrics {
                calls: 5,
                errors: 1,
                total_latency_ms: 2500,
            },
        );
        per_tool.insert(
            "write_file".to_string(),
            ToolMetrics {
                calls: 3,
                errors: 0,
                total_latency_ms: 900,
            },
        );

        let snap = MetricsSnapshot {
            total_model_calls: 10,
            total_tool_calls: 8,
            total_errors: 2,
            total_input_tokens: 1000,
            total_output_tokens: 500,
            avg_model_latency_ms: 250.0,
            per_tool,
        };

        let output = render_prometheus("myapp", &snap);

        // Global counters.
        assert!(output.contains("myapp_model_calls_total 10"));
        assert!(output.contains("myapp_model_latency_seconds 0.250000"));
        assert!(output.contains("myapp_model_errors_total 2"));
        assert!(output.contains("myapp_tokens_input_total 1000"));
        assert!(output.contains("myapp_tokens_output_total 500"));

        // Per-tool: read_file
        assert!(output.contains("myapp_tool_calls_total{tool=\"read_file\"} 5"));
        assert!(output.contains("myapp_tool_errors_total{tool=\"read_file\"} 1"));
        // avg latency = 2500ms / 5 = 500ms = 0.5s
        assert!(output.contains("myapp_tool_latency_seconds{tool=\"read_file\"} 0.500000"));

        // Per-tool: write_file
        assert!(output.contains("myapp_tool_calls_total{tool=\"write_file\"} 3"));
        assert!(output.contains("myapp_tool_errors_total{tool=\"write_file\"} 0"));
        // avg latency = 900ms / 3 = 300ms = 0.3s
        assert!(output.contains("myapp_tool_latency_seconds{tool=\"write_file\"} 0.300000"));

        // HELP and TYPE for tool metrics.
        assert!(output.contains("# HELP myapp_tool_calls_total"));
        assert!(output.contains("# TYPE myapp_tool_calls_total counter"));
        assert!(output.contains("# HELP myapp_tool_errors_total"));
        assert!(output.contains("# TYPE myapp_tool_errors_total counter"));
        assert!(output.contains("# HELP myapp_tool_latency_seconds"));
        assert!(output.contains("# TYPE myapp_tool_latency_seconds gauge"));
    }

    #[tokio::test]
    async fn exporter_render() {
        use synaptic_core::{CallbackHandler, RunEvent};

        let cb = Arc::new(MetricsCallback::new());

        // Simulate a model call.
        cb.on_event(RunEvent::BeforeMessage {
            run_id: "r1".to_string(),
            message_count: 1,
        })
        .await
        .unwrap();
        cb.on_event(RunEvent::AfterMessage {
            run_id: "r1".to_string(),
            response_length: 42,
        })
        .await
        .unwrap();

        // Simulate token recording.
        cb.record_tokens(100, 50).await;

        // Simulate a tool call.
        cb.on_event(RunEvent::BeforeToolCall {
            run_id: "r1".to_string(),
            tool_name: "search".to_string(),
            arguments: "{}".to_string(),
        })
        .await
        .unwrap();
        cb.on_event(RunEvent::AfterToolCall {
            run_id: "r1".to_string(),
            tool_name: "search".to_string(),
            result: "ok".to_string(),
        })
        .await
        .unwrap();

        let exporter = PrometheusExporter::new(cb);
        let output = exporter.render().await;

        assert!(output.contains("synaptic_model_calls_total 1"));
        assert!(output.contains("synaptic_tokens_input_total 100"));
        assert!(output.contains("synaptic_tokens_output_total 50"));
        assert!(output.contains("synaptic_tool_calls_total{tool=\"search\"} 1"));
    }

    #[tokio::test]
    async fn serve_and_stop() {
        let cb = Arc::new(MetricsCallback::new());
        cb.record_tokens(42, 10).await;

        let exporter = PrometheusExporter::new(cb);
        let handle = exporter.serve("127.0.0.1:0").await.unwrap();
        let addr = handle.addr();

        // Fetch /metrics via a plain TCP request.
        let url = format!("http://{}/metrics", addr);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = resp.text().await.unwrap();
        assert!(body.contains("synaptic_tokens_input_total 42"));
        assert!(body.contains("synaptic_tokens_output_total 10"));

        handle.stop_and_wait().await;
    }
}
