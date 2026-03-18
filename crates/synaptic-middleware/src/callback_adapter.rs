#![allow(deprecated)]

use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{CallbackHandler, Message, RunEvent, SynapticError};
use uuid::Uuid;

use crate::{AgentMiddleware, ModelRequest, ModelResponse};

/// Adapts a [`CallbackHandler`] into an [`AgentMiddleware`].
///
/// This allows any callback handler (e.g., `OpenTelemetryCallback`,
/// `RecordingCallback`) to be used in the middleware stack of a Deep Agent
/// or graph agent that only accepts `Vec<Arc<dyn AgentMiddleware>>`.
///
/// The adapter fires lifecycle events at each middleware hook:
/// - `before_agent` → `RunEvent::RunStarted`
/// - `after_agent` → `RunEvent::RunFinished`
/// - `before_model` → `RunEvent::BeforeMessage`
/// - `after_model` → `RunEvent::LlmCalled`
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
pub struct CallbackMiddleware {
    handler: Arc<dyn CallbackHandler>,
    run_id: String,
}

impl CallbackMiddleware {
    /// Create a new adapter wrapping the given callback handler.
    pub fn new(handler: Arc<dyn CallbackHandler>) -> Self {
        Self {
            handler,
            run_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create with a specific run ID (useful for correlating spans).
    pub fn with_run_id(handler: Arc<dyn CallbackHandler>, run_id: String) -> Self {
        Self { handler, run_id }
    }
}

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for CallbackMiddleware {
    async fn before_agent(&self, _messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        self.handler
            .on_event(RunEvent::RunStarted {
                run_id: self.run_id.clone(),
                session_id: String::new(),
            })
            .await
    }

    async fn after_agent(&self, messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        let output = messages
            .last()
            .map(|m| m.content().to_string())
            .unwrap_or_default();
        self.handler
            .on_event(RunEvent::RunFinished {
                run_id: self.run_id.clone(),
                output,
            })
            .await
    }

    async fn before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
        self.handler
            .on_event(RunEvent::BeforeMessage {
                run_id: self.run_id.clone(),
                message_count: request.messages.len(),
            })
            .await
    }

    async fn after_model(
        &self,
        _request: &ModelRequest,
        response: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        self.handler
            .on_event(RunEvent::LlmCalled {
                run_id: self.run_id.clone(),
                message_count: response.message.content().len(),
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use synaptic_core::RunEvent;

    struct RecordHandler {
        events: Mutex<Vec<String>>,
    }

    impl RecordHandler {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CallbackHandler for RecordHandler {
        async fn on_event(&self, event: RunEvent) -> Result<(), SynapticError> {
            let label = match &event {
                RunEvent::RunStarted { .. } => "RunStarted",
                RunEvent::RunFinished { .. } => "RunFinished",
                RunEvent::BeforeMessage { .. } => "BeforeMessage",
                RunEvent::LlmCalled { .. } => "LlmCalled",
                _ => "Other",
            };
            self.events.lock().unwrap().push(label.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn callback_middleware_fires_events() {
        let handler = Arc::new(RecordHandler::new());
        let mw = CallbackMiddleware::new(handler.clone());

        // before_agent
        let mut messages = vec![Message::human("hello")];
        mw.before_agent(&mut messages).await.unwrap();

        // before_model
        let mut req = ModelRequest {
            messages: vec![Message::human("hello")],
            tools: vec![],
            tool_choice: None,
            system_prompt: None,
            thinking: None,
        };
        mw.before_model(&mut req).await.unwrap();

        // after_model
        let mut resp = ModelResponse {
            message: Message::ai("world"),
            usage: None,
        };
        mw.after_model(&req, &mut resp).await.unwrap();

        // after_agent
        messages.push(Message::ai("world"));
        mw.after_agent(&mut messages).await.unwrap();

        let events = handler.events();
        assert_eq!(
            events,
            vec!["RunStarted", "BeforeMessage", "LlmCalled", "RunFinished"]
        );
    }
}
