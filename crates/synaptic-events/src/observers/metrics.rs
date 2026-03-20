//! Metrics collection subscriber — records latency, token counts, and errors.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use synaptic_core::SynapticError;
use tokio::sync::RwLock;

use crate::{Event, EventAction, EventFilter, EventKind, EventSubscriber};

/// Snapshot of collected metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    /// Total model calls.
    pub total_model_calls: u64,
    /// Total tool calls.
    pub total_tool_calls: u64,
    /// Total errors.
    pub total_errors: u64,
    /// Total input tokens across all requests.
    pub total_input_tokens: u64,
    /// Total output tokens across all requests.
    pub total_output_tokens: u64,
    /// Average model call latency in milliseconds.
    pub avg_model_latency_ms: f64,
    /// Per-tool metrics.
    pub per_tool: HashMap<String, ToolMetrics>,
}

/// Metrics for a specific tool.
#[derive(Debug, Clone, Default)]
pub struct ToolMetrics {
    pub calls: u64,
    pub errors: u64,
    pub total_latency_ms: u64,
}

struct MetricsState {
    total_model_calls: u64,
    total_tool_calls: u64,
    total_errors: u64,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_model_latency_ms: u64,
    per_tool: HashMap<String, ToolMetrics>,
    /// Pending model call start times (keyed by request_id).
    model_start_times: HashMap<String, Instant>,
    /// Pending tool call start times (keyed by request_id + tool_name).
    tool_start_times: HashMap<String, Instant>,
}

/// Subscriber that collects latency, token, and error metrics.
///
/// Subscribes to `LlmInput`, `LlmOutput`, `BeforeToolCall`, `AfterToolCall`,
/// and `OnModelError` events to measure model/tool call latency, accumulate
/// token usage, and count errors.
///
/// Call [`snapshot()`](MetricsCallback::snapshot) at any time to read the
/// current metrics, or [`reset()`](MetricsCallback::reset) to zero them out.
pub struct MetricsCallback {
    state: Arc<RwLock<MetricsState>>,
}

impl MetricsCallback {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MetricsState {
                total_model_calls: 0,
                total_tool_calls: 0,
                total_errors: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_model_latency_ms: 0,
                per_tool: HashMap::new(),
                model_start_times: HashMap::new(),
                tool_start_times: HashMap::new(),
            })),
        }
    }

    /// Take a snapshot of the current metrics.
    pub async fn snapshot(&self) -> MetricsSnapshot {
        let state = self.state.read().await;
        let avg = if state.total_model_calls > 0 {
            state.total_model_latency_ms as f64 / state.total_model_calls as f64
        } else {
            0.0
        };
        MetricsSnapshot {
            total_model_calls: state.total_model_calls,
            total_tool_calls: state.total_tool_calls,
            total_errors: state.total_errors,
            total_input_tokens: state.total_input_tokens,
            total_output_tokens: state.total_output_tokens,
            avg_model_latency_ms: avg,
            per_tool: state.per_tool.clone(),
        }
    }

    /// Record token usage externally (e.g. from a `ChatResponse`).
    ///
    /// This allows callers that have access to the actual `TokenUsage` from
    /// model responses to feed it into the metrics.
    pub async fn record_tokens(&self, input_tokens: u64, output_tokens: u64) {
        let mut state = self.state.write().await;
        state.total_input_tokens += input_tokens;
        state.total_output_tokens += output_tokens;
    }

    /// Reset all metrics.
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.total_model_calls = 0;
        state.total_tool_calls = 0;
        state.total_errors = 0;
        state.total_input_tokens = 0;
        state.total_output_tokens = 0;
        state.total_model_latency_ms = 0;
        state.per_tool.clear();
    }
}

impl Default for MetricsCallback {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventSubscriber for MetricsCallback {
    fn subscriptions(&self) -> Vec<EventFilter> {
        vec![EventFilter::AnyOf(vec![
            EventKind::LlmInput,
            EventKind::LlmOutput,
            EventKind::BeforeToolCall,
            EventKind::AfterToolCall,
            EventKind::OnModelError,
        ])]
    }

    async fn handle(&self, event: &mut Event) -> Result<EventAction, SynapticError> {
        let mut state = self.state.write().await;
        let request_id = event.metadata.request_id.clone().unwrap_or_default();

        match event.kind {
            EventKind::LlmInput => {
                state.model_start_times.insert(request_id, Instant::now());
            }
            EventKind::LlmOutput => {
                let elapsed = state
                    .model_start_times
                    .remove(&request_id)
                    .map(|start| start.elapsed().as_millis() as u64)
                    .unwrap_or(0);

                state.total_model_calls += 1;
                state.total_model_latency_ms += elapsed;
            }
            EventKind::BeforeToolCall => {
                let tool_name = event
                    .payload
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let key = format!("{}:{}", request_id, tool_name);
                state.tool_start_times.insert(key, Instant::now());
            }
            EventKind::AfterToolCall => {
                let tool_name = event
                    .payload
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let key = format!("{}:{}", request_id, tool_name);
                let elapsed = state
                    .tool_start_times
                    .remove(&key)
                    .map(|start| start.elapsed().as_millis() as u64)
                    .unwrap_or(0);

                state.total_tool_calls += 1;
                let tool_metrics = state.per_tool.entry(tool_name).or_default();
                tool_metrics.calls += 1;
                tool_metrics.total_latency_ms += elapsed;
            }
            EventKind::OnModelError => {
                state.total_errors += 1;
            }
            _ => {}
        }
        Ok(EventAction::Continue)
    }

    fn name(&self) -> &str {
        "MetricsCallback"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, EventKind};

    fn make_event(kind: EventKind, payload: serde_json::Value) -> Event {
        Event::new(kind, payload).with_request_id("r1")
    }

    #[tokio::test]
    async fn test_metrics_snapshot_empty() {
        let cb = MetricsCallback::new();
        let snap = cb.snapshot().await;
        assert_eq!(snap.total_model_calls, 0);
        assert_eq!(snap.total_tool_calls, 0);
        assert_eq!(snap.total_errors, 0);
    }

    #[tokio::test]
    async fn test_metrics_model_call() {
        let cb = MetricsCallback::new();
        let mut ev_input = make_event(EventKind::LlmInput, serde_json::json!({"message_count": 3}));
        cb.handle(&mut ev_input).await.unwrap();

        // Simulate some latency
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let mut ev_output = make_event(
            EventKind::LlmOutput,
            serde_json::json!({"response_length": 42}),
        );
        cb.handle(&mut ev_output).await.unwrap();

        // Also record token usage externally
        cb.record_tokens(10, 5).await;

        let snap = cb.snapshot().await;
        assert_eq!(snap.total_model_calls, 1);
        assert_eq!(snap.total_input_tokens, 10);
        assert_eq!(snap.total_output_tokens, 5);
        assert!(snap.avg_model_latency_ms >= 5.0);
    }

    #[tokio::test]
    async fn test_metrics_tool_call() {
        let cb = MetricsCallback::new();
        let mut ev_before = make_event(
            EventKind::BeforeToolCall,
            serde_json::json!({"tool_name": "read_file", "arguments": "{}"}),
        );
        cb.handle(&mut ev_before).await.unwrap();

        let mut ev_after = make_event(
            EventKind::AfterToolCall,
            serde_json::json!({"tool_name": "read_file", "result": "ok"}),
        );
        cb.handle(&mut ev_after).await.unwrap();

        let snap = cb.snapshot().await;
        assert_eq!(snap.total_tool_calls, 1);
        assert!(snap.per_tool.contains_key("read_file"));
        assert_eq!(snap.per_tool["read_file"].calls, 1);
    }

    #[tokio::test]
    async fn test_metrics_error_counting() {
        let cb = MetricsCallback::new();
        let mut ev = make_event(
            EventKind::OnModelError,
            serde_json::json!({"error": "oops"}),
        );
        cb.handle(&mut ev).await.unwrap();

        assert_eq!(cb.snapshot().await.total_errors, 1);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let cb = MetricsCallback::new();
        let mut ev = make_event(
            EventKind::OnModelError,
            serde_json::json!({"error": "oops"}),
        );
        cb.handle(&mut ev).await.unwrap();

        assert_eq!(cb.snapshot().await.total_errors, 1);
        cb.reset().await;
        assert_eq!(cb.snapshot().await.total_errors, 0);
    }

    #[tokio::test]
    async fn test_metrics_multiple_tools() {
        let cb = MetricsCallback::new();

        // Two calls to "read_file"
        for i in 0..2 {
            let rid = format!("r{}", i);
            let mut ev_before = Event::new(
                EventKind::BeforeToolCall,
                serde_json::json!({"tool_name": "read_file", "arguments": "{}"}),
            )
            .with_request_id(&rid);
            cb.handle(&mut ev_before).await.unwrap();

            let mut ev_after = Event::new(
                EventKind::AfterToolCall,
                serde_json::json!({"tool_name": "read_file", "result": "ok"}),
            )
            .with_request_id(&rid);
            cb.handle(&mut ev_after).await.unwrap();
        }

        // One call to "write_file"
        let mut ev_before = Event::new(
            EventKind::BeforeToolCall,
            serde_json::json!({"tool_name": "write_file", "arguments": "{}"}),
        )
        .with_request_id("r2");
        cb.handle(&mut ev_before).await.unwrap();

        let mut ev_after = Event::new(
            EventKind::AfterToolCall,
            serde_json::json!({"tool_name": "write_file", "result": "ok"}),
        )
        .with_request_id("r2");
        cb.handle(&mut ev_after).await.unwrap();

        let snap = cb.snapshot().await;
        assert_eq!(snap.total_tool_calls, 3);
        assert_eq!(snap.per_tool["read_file"].calls, 2);
        assert_eq!(snap.per_tool["write_file"].calls, 1);
    }
}
