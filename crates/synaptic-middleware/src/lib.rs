#[cfg(feature = "condenser")]
pub mod condenser;

#[cfg(feature = "plugin")]
mod plugin_hook;
#[cfg(feature = "plugin")]
pub use plugin_hook::PluginHookInterceptor;

mod circuit_breaker;
mod context_editing;
mod human_in_the_loop;
mod model_call_limit;
mod model_fallback;
mod security;
mod ssrf_guard;
mod summarization;
mod todo_list;
mod tool_call_limit;
mod tool_retry;

pub use circuit_breaker::{CircuitBreakerConfig, CircuitBreakerMiddleware, CircuitState};
pub use context_editing::{ContextEditingMiddleware, ContextStrategy};
pub use human_in_the_loop::{ApprovalCallback, HumanInTheLoopMiddleware};
pub use model_call_limit::ModelCallLimitMiddleware;
pub use model_fallback::ModelFallbackMiddleware;
pub use security::{
    ConfirmationPolicy, RiskLevel, RuleBasedAnalyzer, SecurityAnalyzer,
    SecurityConfirmationCallback, SecurityMiddleware, ThresholdConfirmationPolicy,
};
pub use ssrf_guard::{SsrfGuardConfig, SsrfGuardMiddleware};
pub use summarization::SummarizationMiddleware;
pub use todo_list::TodoListMiddleware;
pub use tool_call_limit::ToolCallLimitMiddleware;
pub use tool_retry::ToolRetryMiddleware;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::{
    ChatModel, ChatRequest, ChatResponse, Message, RunContext, SynapticError, ThinkingLevel,
    TokenUsage, ToolCall, ToolChoice, ToolDefinition,
};

// ---------------------------------------------------------------------------
// ModelRequest / ModelResponse — middleware-visible request & response types
// ---------------------------------------------------------------------------

/// A model invocation request visible to middleware.
///
/// Contains all parameters that will be sent to the `ChatModel`, plus
/// the optional system prompt managed by the agent builder.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: Option<ToolChoice>,
    pub system_prompt: Option<String>,
    pub thinking: Option<ThinkingLevel>,
}

impl ModelRequest {
    /// Convert to a `ChatRequest` suitable for calling a `ChatModel`.
    pub fn to_chat_request(&self) -> ChatRequest {
        let mut messages = Vec::new();
        if let Some(ref prompt) = self.system_prompt {
            messages.push(Message::system(prompt));
        }
        messages.extend(self.messages.clone());
        let mut req = ChatRequest::new(messages).with_tools(self.tools.clone());
        if let Some(ref choice) = self.tool_choice {
            req = req.with_tool_choice(choice.clone());
        }
        if let Some(ref thinking) = self.thinking {
            req = req.with_thinking(thinking.clone());
        }
        req
    }
}

/// A model invocation response visible to middleware.
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub message: Message,
    pub usage: Option<TokenUsage>,
}

impl From<ChatResponse> for ModelResponse {
    fn from(resp: ChatResponse) -> Self {
        Self {
            message: resp.message,
            usage: resp.usage,
        }
    }
}

// ---------------------------------------------------------------------------
// ToolCallRequest — wrapper around a single tool call
// ---------------------------------------------------------------------------

/// A single tool call request visible to middleware.
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    pub call: ToolCall,
}

// ---------------------------------------------------------------------------
// File/Shell hook types
// ---------------------------------------------------------------------------

/// Describes a file operation intercepted by middleware.
#[derive(Debug, Clone)]
pub struct FileOp {
    pub path: String,
    pub kind: FileOpKind,
}

/// The kind of file operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOpKind {
    Read,
    Write,
    Delete,
}

/// Result of a file operation.
#[derive(Debug, Clone)]
pub struct FileOpResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Decision for a file operation.
#[derive(Debug, Clone)]
pub enum FileOpDecision {
    /// Allow the operation to proceed.
    Allow,
    /// Deny the operation with a reason.
    Deny(String),
}

/// Describes a shell command intercepted by middleware.
#[derive(Debug, Clone)]
pub struct CommandOp {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
}

/// Result of a command execution.
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Decision for a command execution.
#[derive(Debug, Clone)]
pub enum CommandDecision {
    Allow,
    Deny(String),
}

// ---------------------------------------------------------------------------
// ModelCaller / ToolCaller — "next" in the middleware chain
// ---------------------------------------------------------------------------

/// Trait representing the next step in the model call chain.
///
/// The innermost implementation calls the actual `ChatModel`; outer
/// layers are middleware `wrap_model_call` implementations.
#[async_trait]
pub trait ModelCaller: Send + Sync {
    async fn call(
        &self,
        request: ModelRequest,
        ctx: &RunContext,
    ) -> Result<ModelResponse, SynapticError>;
}

/// Trait representing the next step in the tool call chain.
#[async_trait]
pub trait ToolCaller: Send + Sync {
    async fn call(&self, request: ToolCallRequest) -> Result<Value, SynapticError>;
}

// ---------------------------------------------------------------------------
// Interceptor trait
// ---------------------------------------------------------------------------

/// A lightweight interceptor for wrapping model and tool calls.
///
/// Provides `before_model`, `wrap_model_call`, `after_model`, and
/// `wrap_tool_call` with no-op defaults.
///
/// # Lifecycle order
///
/// ```text
/// loop {
///   before_model (forward)  →  wrap_model_call (onion)  →  after_model (reverse)
///   for each tool_call { wrap_tool_call (onion) }
/// }
/// ```
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// Human-readable name for diagnostics and UI display.
    /// Defaults to the fully-qualified type name.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Called before each model invocation. Can modify the request.
    /// Runs in forward order (first added → first called).
    async fn before_model(&self, _req: &mut ModelRequest) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Called after each model invocation. Can modify the response.
    /// Runs in reverse order (last added → first called).
    async fn after_model(
        &self,
        _req: &ModelRequest,
        _resp: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Wrap a model call. Override to intercept or modify the request/response.
    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        ctx: &RunContext,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        next.call(request, ctx).await
    }

    /// Wrap a tool call. Override to intercept or modify tool execution.
    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        next.call(request).await
    }
}

// ---------------------------------------------------------------------------
// InterceptorChain — composes multiple Interceptors
// ---------------------------------------------------------------------------

/// A chain of [`Interceptor`]s that executes them in the correct lifecycle order.
///
/// `call_model()` runs: all `before_model` (forward) → `wrap_model_call` onion
/// chain → all `after_model` (reverse).
///
/// `call_tool()` runs the `wrap_tool_call` onion chain.
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    pub fn new(interceptors: Vec<Arc<dyn Interceptor>>) -> Self {
        Self { interceptors }
    }

    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }

    /// Execute a model call through the full interceptor chain.
    ///
    /// Runs the complete lifecycle: `before_model` (forward) →
    /// `wrap_model_call` onion chain → `after_model` (reverse).
    pub async fn call_model(
        &self,
        mut request: ModelRequest,
        ctx: &RunContext,
        base: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        // Run before_model hooks in forward order
        for interceptor in &self.interceptors {
            interceptor.before_model(&mut request).await?;
        }

        // Build the wrapped call chain (outermost first)
        let chain = InterceptorWrapModelChain {
            interceptors: &self.interceptors,
            index: 0,
            ctx,
            base,
        };
        let mut response = chain.call(request.clone(), ctx).await?;

        // Run after_model hooks in reverse order
        for interceptor in self.interceptors.iter().rev() {
            interceptor.after_model(&request, &mut response).await?;
        }

        Ok(response)
    }

    /// Execute a tool call through the full interceptor chain.
    pub async fn call_tool(
        &self,
        request: ToolCallRequest,
        base: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        let chain = InterceptorWrapToolChain {
            interceptors: &self.interceptors,
            index: 0,
            base,
        };
        chain.call(request).await
    }
}

// Internal chain helpers for Interceptor-based recursive wrap_model_call / wrap_tool_call

struct InterceptorWrapModelChain<'a> {
    interceptors: &'a [Arc<dyn Interceptor>],
    index: usize,
    ctx: &'a RunContext,
    base: &'a dyn ModelCaller,
}

#[async_trait]
impl ModelCaller for InterceptorWrapModelChain<'_> {
    async fn call(
        &self,
        request: ModelRequest,
        ctx: &RunContext,
    ) -> Result<ModelResponse, SynapticError> {
        if self.index >= self.interceptors.len() {
            self.base.call(request, ctx).await
        } else {
            let next = InterceptorWrapModelChain {
                interceptors: self.interceptors,
                index: self.index + 1,
                ctx: self.ctx,
                base: self.base,
            };
            self.interceptors[self.index]
                .wrap_model_call(request, ctx, &next)
                .await
        }
    }
}

struct InterceptorWrapToolChain<'a> {
    interceptors: &'a [Arc<dyn Interceptor>],
    index: usize,
    base: &'a dyn ToolCaller,
}

#[async_trait]
impl ToolCaller for InterceptorWrapToolChain<'_> {
    async fn call(&self, request: ToolCallRequest) -> Result<Value, SynapticError> {
        if self.index >= self.interceptors.len() {
            self.base.call(request).await
        } else {
            let next = InterceptorWrapToolChain {
                interceptors: self.interceptors,
                index: self.index + 1,
                base: self.base,
            };
            self.interceptors[self.index]
                .wrap_tool_call(request, &next)
                .await
        }
    }
}

// ---------------------------------------------------------------------------
// BaseChatModelCaller — calls the actual ChatModel
// ---------------------------------------------------------------------------

/// Wraps a `ChatModel` into a `ModelCaller`.
pub struct BaseChatModelCaller {
    model: Arc<dyn ChatModel>,
}

impl BaseChatModelCaller {
    pub fn new(model: Arc<dyn ChatModel>) -> Self {
        Self { model }
    }
}

#[async_trait]
impl ModelCaller for BaseChatModelCaller {
    async fn call(
        &self,
        request: ModelRequest,
        _ctx: &RunContext,
    ) -> Result<ModelResponse, SynapticError> {
        let chat_request = request.to_chat_request();
        let response = self.model.chat(chat_request).await?;
        Ok(response.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_request_to_chat_request() {
        let req = ModelRequest {
            messages: vec![Message::human("hello")],
            tools: vec![],
            tool_choice: None,
            system_prompt: Some("You are helpful.".to_string()),
            thinking: None,
        };
        let chat_req = req.to_chat_request();
        assert_eq!(chat_req.messages.len(), 2);
        assert!(chat_req.messages[0].is_system());
        assert!(chat_req.messages[1].is_human());
    }

    #[test]
    fn model_request_without_system_prompt() {
        let req = ModelRequest {
            messages: vec![Message::human("hello")],
            tools: vec![],
            tool_choice: None,
            system_prompt: None,
            thinking: None,
        };
        let chat_req = req.to_chat_request();
        assert_eq!(chat_req.messages.len(), 1);
    }

    use std::sync::Mutex;

    /// A mock model caller that returns a fixed response.
    struct MockModelCaller;

    #[async_trait]
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

    /// A mock tool caller that returns a fixed JSON value.
    struct MockToolCaller;

    #[async_trait]
    impl ToolCaller for MockToolCaller {
        async fn call(&self, _request: ToolCallRequest) -> Result<Value, SynapticError> {
            Ok(serde_json::json!({"result": "ok"}))
        }
    }

    /// An interceptor that records the order of lifecycle calls.
    struct OrderTrackingInterceptor {
        id: usize,
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Interceptor for OrderTrackingInterceptor {
        async fn before_model(&self, _req: &mut ModelRequest) -> Result<(), SynapticError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("before_model:{}", self.id));
            Ok(())
        }

        async fn after_model(
            &self,
            _req: &ModelRequest,
            _resp: &mut ModelResponse,
        ) -> Result<(), SynapticError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("after_model:{}", self.id));
            Ok(())
        }

        async fn wrap_model_call(
            &self,
            request: ModelRequest,
            ctx: &RunContext,
            next: &dyn ModelCaller,
        ) -> Result<ModelResponse, SynapticError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("wrap_model_enter:{}", self.id));
            let resp = next.call(request, ctx).await?;
            self.log
                .lock()
                .unwrap()
                .push(format!("wrap_model_exit:{}", self.id));
            Ok(resp)
        }
    }

    fn make_model_request() -> ModelRequest {
        ModelRequest {
            messages: vec![Message::human("test")],
            tools: vec![],
            tool_choice: None,
            system_prompt: None,
            thinking: None,
        }
    }

    #[tokio::test]
    async fn interceptor_chain_lifecycle_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let interceptors: Vec<Arc<dyn Interceptor>> = (1..=3)
            .map(|id| {
                Arc::new(OrderTrackingInterceptor {
                    id,
                    log: log.clone(),
                }) as Arc<dyn Interceptor>
            })
            .collect();

        let chain = InterceptorChain::new(interceptors);
        let base = MockModelCaller;
        chain
            .call_model(make_model_request(), &RunContext::default(), &base)
            .await
            .unwrap();

        let entries = log.lock().unwrap().clone();
        assert_eq!(
            entries,
            vec![
                // before_model: forward order
                "before_model:1",
                "before_model:2",
                "before_model:3",
                // wrap_model_call: onion enter (forward)
                "wrap_model_enter:1",
                "wrap_model_enter:2",
                "wrap_model_enter:3",
                // wrap_model_call: onion exit (reverse)
                "wrap_model_exit:3",
                "wrap_model_exit:2",
                "wrap_model_exit:1",
                // after_model: reverse order
                "after_model:3",
                "after_model:2",
                "after_model:1",
            ]
        );
    }

    #[tokio::test]
    async fn interceptor_chain_empty_passthrough() {
        let chain = InterceptorChain::new(vec![]);
        assert!(chain.is_empty());

        let base = MockModelCaller;
        let resp = chain
            .call_model(make_model_request(), &RunContext::default(), &base)
            .await
            .unwrap();
        assert_eq!(resp.message.content(), "mock response");
    }

    #[tokio::test]
    async fn interceptor_chain_empty_tool_passthrough() {
        let chain = InterceptorChain::new(vec![]);
        let base = MockToolCaller;
        let request = ToolCallRequest {
            call: ToolCall {
                id: "test".to_string(),
                name: "test_tool".to_string(),
                arguments: serde_json::json!({}),
            },
        };
        let result = chain.call_tool(request, &base).await.unwrap();
        assert_eq!(result, serde_json::json!({"result": "ok"}));
    }

    /// An interceptor whose before_model always fails.
    struct FailingBeforeModelInterceptor;

    #[async_trait]
    impl Interceptor for FailingBeforeModelInterceptor {
        async fn before_model(&self, _req: &mut ModelRequest) -> Result<(), SynapticError> {
            Err(SynapticError::Validation("before_model failed".to_string()))
        }
    }

    #[tokio::test]
    async fn interceptor_chain_before_model_error_aborts() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let interceptors: Vec<Arc<dyn Interceptor>> = vec![
            Arc::new(FailingBeforeModelInterceptor),
            Arc::new(OrderTrackingInterceptor {
                id: 2,
                log: log.clone(),
            }),
        ];

        let chain = InterceptorChain::new(interceptors);
        let base = MockModelCaller;
        let result = chain
            .call_model(make_model_request(), &RunContext::default(), &base)
            .await;

        assert!(result.is_err());
        // MW2's before_model should never run
        let entries = log.lock().unwrap().clone();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn interceptor_chain_tool_onion_order() {
        let log = Arc::new(Mutex::new(Vec::new()));

        struct ToolOrderInterceptor {
            id: usize,
            log: Arc<Mutex<Vec<String>>>,
        }

        #[async_trait]
        impl Interceptor for ToolOrderInterceptor {
            async fn wrap_tool_call(
                &self,
                request: ToolCallRequest,
                next: &dyn ToolCaller,
            ) -> Result<Value, SynapticError> {
                self.log
                    .lock()
                    .unwrap()
                    .push(format!("tool_enter:{}", self.id));
                let result = next.call(request).await?;
                self.log
                    .lock()
                    .unwrap()
                    .push(format!("tool_exit:{}", self.id));
                Ok(result)
            }
        }

        let interceptors: Vec<Arc<dyn Interceptor>> = (1..=3)
            .map(|id| {
                Arc::new(ToolOrderInterceptor {
                    id,
                    log: log.clone(),
                }) as Arc<dyn Interceptor>
            })
            .collect();

        let chain = InterceptorChain::new(interceptors);
        let base = MockToolCaller;
        let request = ToolCallRequest {
            call: ToolCall {
                id: "t1".to_string(),
                name: "test".to_string(),
                arguments: serde_json::json!({}),
            },
        };
        chain.call_tool(request, &base).await.unwrap();

        let entries = log.lock().unwrap().clone();
        assert_eq!(
            entries,
            vec![
                "tool_enter:1",
                "tool_enter:2",
                "tool_enter:3",
                "tool_exit:3",
                "tool_exit:2",
                "tool_exit:1",
            ]
        );
    }
}
