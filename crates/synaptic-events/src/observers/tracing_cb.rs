use async_trait::async_trait;
use synaptic_core::SynapticError;

use crate::{Event, EventAction, EventFilter, EventKind, EventSubscriber};

/// An event subscriber that logs all events via the `tracing` crate.
pub struct TracingCallback;

impl TracingCallback {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TracingCallback {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventSubscriber for TracingCallback {
    fn subscriptions(&self) -> Vec<EventFilter> {
        vec![EventFilter::All]
    }

    async fn handle(&self, event: &mut Event) -> Result<EventAction, SynapticError> {
        let request_id = event.metadata.request_id.as_deref().unwrap_or("?");
        let source = &event.metadata.source;

        match event.kind {
            EventKind::SessionStart => {
                tracing::info!(request_id = %request_id, source = %source, "session started");
            }
            EventKind::AgentEnd => {
                tracing::info!(request_id = %request_id, source = %source, "agent ended");
            }
            EventKind::LlmInput => {
                let message_count = event
                    .payload
                    .get("message_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                tracing::info!(request_id = %request_id, message_count = message_count, "before message");
            }
            EventKind::LlmOutput => {
                let response_length = event
                    .payload
                    .get("response_length")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                tracing::info!(request_id = %request_id, response_length = response_length, "LLM output");
            }
            EventKind::BeforeToolCall => {
                let tool_name = event
                    .payload
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                tracing::info!(request_id = %request_id, tool_name = %tool_name, "before tool call");
            }
            EventKind::AfterToolCall => {
                let tool_name = event
                    .payload
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                tracing::info!(request_id = %request_id, tool_name = %tool_name, "after tool call");
            }
            EventKind::OnModelError | EventKind::OnToolError => {
                let error = event
                    .payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                tracing::error!(request_id = %request_id, error = %error, kind = %event.kind, "error event");
            }
            _ => {
                tracing::info!(request_id = %request_id, kind = %event.kind, source = %source, "event");
            }
        }
        Ok(EventAction::Continue)
    }

    fn name(&self) -> &str {
        "TracingCallback"
    }
}
