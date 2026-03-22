//! PluginHookInterceptor — bridges EventBus lifecycle events into the Interceptor pipeline.
//!
//! Emits structured events to the [`EventBus`] at each agent lifecycle point so
//! that plugin subscribers can observe or influence model and tool calls without
//! coupling directly to the middleware chain.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::SynapticError;
use synaptic_events::{EmitResult, Event, EventBus, EventKind};
use tracing::warn;

use crate::{Interceptor, ModelRequest, ModelResponse, ToolCallRequest, ToolCaller};

// ---------------------------------------------------------------------------
// PluginHookInterceptor
// ---------------------------------------------------------------------------

/// An [`Interceptor`] that bridges the middleware pipeline to the [`EventBus`].
///
/// When added to an [`InterceptorChain`], it emits structured lifecycle events
/// so that plugin subscribers can observe or cancel model and tool calls.
///
/// # Events emitted
///
/// | Lifecycle point     | `EventKind`          | Dispatch      |
/// |---------------------|----------------------|---------------|
/// | `before_model`      | `BeforeModelCall`    | Intercept     |
/// | `after_model`       | `LlmOutput`          | Parallel      |
/// | `wrap_tool_call` (before) | `BeforeToolCall` | Intercept  |
/// | `wrap_tool_call` (after)  | `AfterToolCall`  | Parallel   |
///
/// For `Intercept`-mode events, if a subscriber returns `Cancelled` or
/// `Intercepted`, a warning is logged but the pipeline continues normally —
/// actual cancellation logic should be implemented by the caller.
pub struct PluginHookInterceptor {
    event_bus: Arc<EventBus>,
}

impl PluginHookInterceptor {
    /// Creates a new interceptor backed by the given [`EventBus`].
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self { event_bus }
    }
}

#[async_trait]
impl Interceptor for PluginHookInterceptor {
    /// Emits `BeforeModelCall` before each model invocation.
    ///
    /// Payload: `{ message_count, tool_count, has_system_prompt }`
    async fn before_model(&self, req: &mut ModelRequest) -> Result<(), SynapticError> {
        let payload = serde_json::json!({
            "message_count": req.messages.len(),
            "tool_count": req.tools.len(),
            "has_system_prompt": req.system_prompt.is_some(),
        });
        let mut event =
            Event::new(EventKind::BeforeModelCall, payload).with_source("PluginHookInterceptor");

        match self.event_bus.emit(&mut event).await {
            Ok(EmitResult::Proceed) => {}
            Ok(EmitResult::Cancelled) => {
                warn!("BeforeModelCall event was cancelled by a subscriber; continuing anyway");
            }
            Ok(EmitResult::Intercepted(_)) => {
                warn!("BeforeModelCall event was intercepted by a subscriber; continuing anyway");
            }
            Ok(EmitResult::Retry) => {}
            Err(e) => {
                warn!(error = %e, "BeforeModelCall event emission failed; continuing anyway");
            }
        }
        Ok(())
    }

    /// Emits `LlmOutput` after each model invocation.
    ///
    /// Payload: `{ role, has_usage }`
    async fn after_model(
        &self,
        _req: &ModelRequest,
        resp: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        let payload = serde_json::json!({
            "role": resp.message.role().to_string(),
            "has_usage": resp.usage.is_some(),
        });
        let mut event =
            Event::new(EventKind::LlmOutput, payload).with_source("PluginHookInterceptor");

        // LlmOutput is Parallel — fire and forget; errors are logged by the bus.
        let _ = self.event_bus.emit(&mut event).await;
        Ok(())
    }

    /// Emits `BeforeToolCall` before and `AfterToolCall` after each tool execution.
    ///
    /// Payload for both: `{ tool_name, tool_call_id }`
    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        let tool_name = request.call.name.clone();
        let tool_call_id = request.call.id.clone();

        // Emit BeforeToolCall (Intercept mode).
        let before_payload = serde_json::json!({
            "tool_name": tool_name,
            "tool_call_id": tool_call_id,
        });
        let mut before_event = Event::new(EventKind::BeforeToolCall, before_payload)
            .with_source("PluginHookInterceptor");

        match self.event_bus.emit(&mut before_event).await {
            Ok(EmitResult::Proceed) => {}
            Ok(EmitResult::Cancelled) => {
                warn!(
                    tool = %tool_name,
                    "BeforeToolCall event was cancelled by a subscriber; continuing anyway"
                );
            }
            Ok(EmitResult::Intercepted(_)) => {
                warn!(
                    tool = %tool_name,
                    "BeforeToolCall event was intercepted by a subscriber; continuing anyway"
                );
            }
            Ok(EmitResult::Retry) => {}
            Err(e) => {
                warn!(
                    tool = %tool_name,
                    error = %e,
                    "BeforeToolCall event emission failed; continuing anyway"
                );
            }
        }

        // Execute the actual tool call.
        let result = next.call(request).await;

        // Emit AfterToolCall (Parallel mode) — fire and forget.
        let after_payload = serde_json::json!({
            "tool_name": tool_name,
            "tool_call_id": tool_call_id,
        });
        let mut after_event = Event::new(EventKind::AfterToolCall, after_payload)
            .with_source("PluginHookInterceptor");
        let _ = self.event_bus.emit(&mut after_event).await;

        result
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    use synaptic_core::{Message, RunContext, SynapticError, ToolCall};
    use synaptic_events::{Event, EventAction, EventBus, EventFilter, EventKind, EventSubscriber};

    use super::*;
    use crate::{
        InterceptorChain, ModelCaller, ModelRequest, ModelResponse, ToolCallRequest, ToolCaller,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_model_request() -> ModelRequest {
        ModelRequest {
            messages: vec![Message::human("test")],
            tools: vec![],
            tool_choice: None,
            system_prompt: None,
            thinking: None,
        }
    }

    struct MockModelCaller;

    #[async_trait::async_trait]
    impl ModelCaller for MockModelCaller {
        async fn call(
            &self,
            _request: ModelRequest,
            _ctx: &RunContext,
        ) -> Result<ModelResponse, SynapticError> {
            Ok(ModelResponse {
                message: Message::ai("mock response"),
                usage: None,
            })
        }
    }

    struct MockToolCaller;

    #[async_trait::async_trait]
    impl ToolCaller for MockToolCaller {
        async fn call(
            &self,
            _request: ToolCallRequest,
        ) -> Result<serde_json::Value, SynapticError> {
            Ok(serde_json::json!({"result": "ok"}))
        }
    }

    /// A subscriber that counts how many times it receives a specific event kind.
    struct CountingSubscriber {
        count: Arc<AtomicU32>,
        kind: EventKind,
    }

    #[async_trait::async_trait]
    impl EventSubscriber for CountingSubscriber {
        fn subscriptions(&self) -> Vec<EventFilter> {
            vec![EventFilter::Exact(self.kind)]
        }

        async fn handle(&self, _event: &mut Event) -> Result<EventAction, SynapticError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(EventAction::Continue)
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn hook_interceptor_fires_before_model_call() {
        let bus = Arc::new(EventBus::new());
        let count = Arc::new(AtomicU32::new(0));

        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: count.clone(),
                kind: EventKind::BeforeModelCall,
            }),
            0,
            "test-before-model",
        );

        let interceptor = Arc::new(PluginHookInterceptor::new(Arc::clone(&bus)));
        let chain = InterceptorChain::new(vec![interceptor]);
        let base = MockModelCaller;

        chain
            .call_model(make_model_request(), &RunContext::default(), &base)
            .await
            .expect("model call should succeed");

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "BeforeModelCall should have been emitted once"
        );
    }

    #[tokio::test]
    async fn hook_interceptor_fires_after_model_call() {
        let bus = Arc::new(EventBus::new());
        let count = Arc::new(AtomicU32::new(0));

        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: count.clone(),
                kind: EventKind::LlmOutput,
            }),
            0,
            "test-llm-output",
        );

        let interceptor = Arc::new(PluginHookInterceptor::new(Arc::clone(&bus)));
        let chain = InterceptorChain::new(vec![interceptor]);
        let base = MockModelCaller;

        chain
            .call_model(make_model_request(), &RunContext::default(), &base)
            .await
            .expect("model call should succeed");

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "LlmOutput should have been emitted once"
        );
    }

    #[tokio::test]
    async fn hook_interceptor_fires_before_and_after_tool_call() {
        let bus = Arc::new(EventBus::new());
        let before_count = Arc::new(AtomicU32::new(0));
        let after_count = Arc::new(AtomicU32::new(0));

        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: before_count.clone(),
                kind: EventKind::BeforeToolCall,
            }),
            0,
            "test-before-tool",
        );
        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: after_count.clone(),
                kind: EventKind::AfterToolCall,
            }),
            0,
            "test-after-tool",
        );

        let interceptor = Arc::new(PluginHookInterceptor::new(Arc::clone(&bus)));
        let chain = InterceptorChain::new(vec![interceptor]);
        let base = MockToolCaller;

        let request = ToolCallRequest {
            call: ToolCall {
                id: "call-1".to_string(),
                name: "test_tool".to_string(),
                arguments: serde_json::json!({}),
            },
        };

        chain
            .call_tool(request, &base)
            .await
            .expect("tool call should succeed");

        assert_eq!(
            before_count.load(Ordering::SeqCst),
            1,
            "BeforeToolCall should have been emitted once"
        );
        assert_eq!(
            after_count.load(Ordering::SeqCst),
            1,
            "AfterToolCall should have been emitted once"
        );
    }
}
