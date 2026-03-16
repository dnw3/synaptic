//! Bridge between [`EventBus`] and [`AgentMiddleware`].
//!
//! Emits `EventKind::BeforeModelCall`, `LlmOutput`, `BeforeToolCall`,
//! `AfterToolCall`, and `AgentEnd` events at the appropriate lifecycle
//! points, allowing subscribers to observe — and optionally intercept —
//! model and tool calls.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::SynapticError;
use synaptic_events::{EmitResult, Event, EventBus, EventKind};
use synaptic_middleware::{
    AgentMiddleware, ModelCaller, ModelRequest, ModelResponse, ToolCallRequest, ToolCaller,
};

/// Middleware that bridges the [`EventBus`] into the agent middleware chain.
///
/// This is a transitional integration: rather than modifying the graph's
/// internal node implementations, we expose event emit-points through the
/// existing `AgentMiddleware` hooks (`wrap_model_call`, `wrap_tool_call`,
/// `after_agent`).
pub struct EventBusMiddleware {
    bus: Arc<EventBus>,
}

impl EventBusMiddleware {
    /// Create a new bridge middleware backed by the given `EventBus`.
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self { bus }
    }
}

#[async_trait]
impl AgentMiddleware for EventBusMiddleware {
    // -- model call: before + after ----------------------------------------

    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        // --- BeforeModelCall (Intercept) ---
        let payload = json!({
            "message_count": request.messages.len(),
            "tool_count": request.tools.len(),
        });
        let mut event = Event::new(EventKind::BeforeModelCall, payload).with_source("deep_agent");

        match self.bus.emit(&mut event).await? {
            EmitResult::Intercepted(val) => {
                // Subscriber provided a synthetic response — deserialize it
                // into a ModelResponse so the agent loop can continue.
                let text = val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| val.to_string());
                let message = synaptic_core::Message::ai(&text);
                return Ok(ModelResponse {
                    message,
                    usage: None,
                });
            }
            EmitResult::Cancelled => {
                return Err(SynapticError::Tool(
                    "BeforeModelCall cancelled by event subscriber".to_string(),
                ));
            }
            _ => {}
        }

        // --- Actual model call ---
        let response = next.call(request).await?;

        // --- LlmOutput (Parallel / fire-and-forget) ---
        let content_preview: String = response.message.content().chars().take(500).collect();
        let tool_call_count = response.message.tool_calls().len();
        let output_payload = json!({
            "content_preview": content_preview,
            "tool_call_count": tool_call_count,
            "usage": response.usage.as_ref().map(|u| json!({
                "input_tokens": u.input_tokens,
                "output_tokens": u.output_tokens,
                "total_tokens": u.total_tokens,
            })),
        });
        let mut output_event =
            Event::new(EventKind::LlmOutput, output_payload).with_source("deep_agent");
        // Fire-and-forget — ignore errors from parallel subscribers.
        let _ = self.bus.emit(&mut output_event).await;

        Ok(response)
    }

    // -- tool call: before + after -----------------------------------------

    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        // --- BeforeToolCall (Intercept) ---
        let payload = json!({
            "tool_name": request.call.name,
            "tool_call_id": request.call.id,
            "arguments": request.call.arguments,
        });
        let mut event = Event::new(EventKind::BeforeToolCall, payload).with_source("deep_agent");

        match self.bus.emit(&mut event).await? {
            EmitResult::Intercepted(val) => {
                // Subscriber short-circuited the tool call with a result.
                return Ok(val);
            }
            EmitResult::Cancelled => {
                return Ok(json!({
                    "error": "Tool call cancelled by event subscriber"
                }));
            }
            _ => {}
        }

        // --- Actual tool call ---
        let result = next.call(request.clone()).await;

        // --- AfterToolCall (Parallel / fire-and-forget) ---
        let after_payload = json!({
            "tool_name": request.call.name,
            "tool_call_id": request.call.id,
            "success": result.is_ok(),
        });
        let mut after_event =
            Event::new(EventKind::AfterToolCall, after_payload).with_source("deep_agent");
        let _ = self.bus.emit(&mut after_event).await;

        result
    }

    // -- agent lifecycle ---------------------------------------------------

    async fn after_agent(
        &self,
        messages: &mut Vec<synaptic_core::Message>,
    ) -> Result<(), SynapticError> {
        let payload = json!({
            "message_count": messages.len(),
        });
        let mut event = Event::new(EventKind::AgentEnd, payload).with_source("deep_agent");
        // Fire-and-forget
        let _ = self.bus.emit(&mut event).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use synaptic_events::{EventAction, EventFilter, EventSubscriber};

    struct Counter {
        count: Arc<AtomicU32>,
        kind: EventKind,
    }

    #[async_trait]
    impl EventSubscriber for Counter {
        fn subscriptions(&self) -> Vec<EventFilter> {
            vec![EventFilter::Exact(self.kind)]
        }
        async fn handle(&self, _: &mut Event) -> Result<EventAction, SynapticError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(EventAction::Continue)
        }
    }

    #[tokio::test]
    async fn after_agent_emits_agent_end() {
        let bus = Arc::new(EventBus::new());
        let count = Arc::new(AtomicU32::new(0));
        bus.subscribe(
            Arc::new(Counter {
                count: count.clone(),
                kind: EventKind::AgentEnd,
            }),
            0,
            "test",
        );
        let mw = EventBusMiddleware::new(bus);
        let mut messages = vec![synaptic_core::Message::human("hi")];
        mw.after_agent(&mut messages).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
