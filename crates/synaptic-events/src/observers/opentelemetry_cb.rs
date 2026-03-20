//! OpenTelemetry event subscriber for Synaptic.
//!
//! Emits spans to any OTel-compatible backend via the global OTel tracer.

use async_trait::async_trait;
use opentelemetry::{
    global,
    trace::{Span, Tracer},
    KeyValue,
};
use synaptic_core::SynapticError;

use crate::{Event, EventAction, EventFilter, EventKind, EventSubscriber};

/// Event subscriber that records Synaptic events as OpenTelemetry spans.
///
/// Subscribes to `LlmOutput`, `AfterToolCall`, `SessionStart`, and `AgentEnd`.
pub struct OpenTelemetryCallback {
    service_name: String,
}

impl OpenTelemetryCallback {
    /// Create a new OpenTelemetry subscriber with the given service name.
    ///
    /// Uses the global OTel tracer. Set up your OTel provider before calling.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }
}

#[async_trait]
impl EventSubscriber for OpenTelemetryCallback {
    fn subscriptions(&self) -> Vec<EventFilter> {
        vec![EventFilter::AnyOf(vec![
            EventKind::SessionStart,
            EventKind::AgentEnd,
            EventKind::LlmInput,
            EventKind::LlmOutput,
            EventKind::BeforeToolCall,
            EventKind::AfterToolCall,
            EventKind::OnModelError,
        ])]
    }

    async fn handle(&self, event: &mut Event) -> Result<EventAction, SynapticError> {
        let tracer = global::tracer(self.service_name.clone());
        let request_id = event.metadata.request_id.clone().unwrap_or_default();

        match event.kind {
            EventKind::SessionStart => {
                let mut span = tracer
                    .span_builder("synaptic.session_start")
                    .with_attributes(vec![KeyValue::new("synaptic.request_id", request_id)])
                    .start(&tracer);
                span.end();
            }
            EventKind::LlmInput => {
                let message_count = event
                    .payload
                    .get("message_count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let mut span = tracer
                    .span_builder("synaptic.llm_input")
                    .with_attributes(vec![
                        KeyValue::new("synaptic.request_id", request_id),
                        KeyValue::new("llm.message_count", message_count),
                    ])
                    .start(&tracer);
                span.end();
            }
            EventKind::LlmOutput => {
                let message_count = event
                    .payload
                    .get("message_count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let mut span = tracer
                    .span_builder("synaptic.llm_output")
                    .with_attributes(vec![
                        KeyValue::new("synaptic.request_id", request_id),
                        KeyValue::new("llm.message_count", message_count),
                    ])
                    .start(&tracer);
                span.end();
            }
            EventKind::BeforeToolCall => {
                let tool_name = event
                    .payload
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut span = tracer
                    .span_builder(format!("tool.before.{}", tool_name))
                    .with_attributes(vec![
                        KeyValue::new("synaptic.request_id", request_id),
                        KeyValue::new("tool.name", tool_name),
                    ])
                    .start(&tracer);
                span.end();
            }
            EventKind::AfterToolCall => {
                let tool_name = event
                    .payload
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut span = tracer
                    .span_builder(format!("tool.after.{}", tool_name))
                    .with_attributes(vec![
                        KeyValue::new("synaptic.request_id", request_id),
                        KeyValue::new("tool.name", tool_name),
                    ])
                    .start(&tracer);
                span.end();
            }
            EventKind::AgentEnd => {
                let mut span = tracer
                    .span_builder("synaptic.agent_end")
                    .with_attributes(vec![KeyValue::new("synaptic.request_id", request_id)])
                    .start(&tracer);
                span.end();
            }
            EventKind::OnModelError => {
                let error = event
                    .payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut span = tracer
                    .span_builder("synaptic.error")
                    .with_attributes(vec![
                        KeyValue::new("synaptic.request_id", request_id),
                        KeyValue::new("error.message", error),
                    ])
                    .start(&tracer);
                span.end();
            }
            _ => {}
        }
        Ok(EventAction::Continue)
    }

    fn name(&self) -> &str {
        "OpenTelemetryCallback"
    }
}
