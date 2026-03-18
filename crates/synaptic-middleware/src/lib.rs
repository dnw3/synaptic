#![allow(deprecated)]

mod callback_adapter;
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

pub use callback_adapter::CallbackMiddleware;
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
    ChatModel, ChatRequest, ChatResponse, Message, SynapticError, ThinkingConfig, TokenUsage,
    ToolCall, ToolChoice, ToolDefinition,
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
    pub thinking: Option<ThinkingConfig>,
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
    async fn call(&self, request: ModelRequest) -> Result<ModelResponse, SynapticError>;
}

/// Trait representing the next step in the tool call chain.
#[async_trait]
pub trait ToolCaller: Send + Sync {
    async fn call(&self, request: ToolCallRequest) -> Result<Value, SynapticError>;
}

// ---------------------------------------------------------------------------
// AgentMiddleware trait
// ---------------------------------------------------------------------------

/// Middleware that can intercept and modify agent lifecycle events.
///
/// All methods have default no-op implementations, so middleware only
/// needs to override the hooks it cares about.
///
/// # Lifecycle order
///
/// ```text
/// before_agent
///   loop {
///     before_model  ->  wrap_model_call  ->  after_model
///     for each tool_call { wrap_tool_call }
///   }
/// after_agent
/// ```
///
/// # Deprecation
///
/// This trait is deprecated. Use [`synaptic_events::EventSubscriber`] instead.
/// The `EventBusMiddleware` bridges EventBus events into the agent execution loop,
/// replacing the need for direct middleware implementations.
/// This trait will be removed in a future version.
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
#[async_trait]
pub trait AgentMiddleware: Send + Sync {
    /// Called once when the agent starts executing.
    async fn before_agent(&self, _messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Called once when the agent finishes executing.
    async fn after_agent(&self, _messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Called before each model invocation. Can modify the request.
    async fn before_model(&self, _request: &mut ModelRequest) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Called after each model invocation. Can modify the response.
    async fn after_model(
        &self,
        _request: &ModelRequest,
        _response: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Wraps the model call. Override to intercept or replace the model invocation.
    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        next.call(request).await
    }

    /// Wraps a tool call. Override to intercept or replace tool execution.
    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        next.call(request).await
    }

    /// Called before a file operation. Return `Deny` to block it.
    async fn before_file_op(&self, _op: &FileOp) -> Result<FileOpDecision, SynapticError> {
        Ok(FileOpDecision::Allow)
    }

    /// Called after a file operation completes.
    async fn after_file_op(
        &self,
        _op: &FileOp,
        _result: &FileOpResult,
    ) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Called before a shell command. Return `Deny` to block it.
    async fn before_command(&self, _cmd: &CommandOp) -> Result<CommandDecision, SynapticError> {
        Ok(CommandDecision::Allow)
    }

    /// Called after a shell command completes.
    async fn after_command(
        &self,
        _cmd: &CommandOp,
        _result: &CommandResult,
    ) -> Result<(), SynapticError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MiddlewareChain — composes multiple middlewares
// ---------------------------------------------------------------------------

/// A chain of middlewares that executes them in order.
#[allow(deprecated)]
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn AgentMiddleware>>,
}

#[allow(deprecated)]
impl MiddlewareChain {
    pub fn new(middlewares: Vec<Arc<dyn AgentMiddleware>>) -> Self {
        Self { middlewares }
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub async fn run_before_agent(&self, messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        for mw in &self.middlewares {
            mw.before_agent(messages).await?;
        }
        Ok(())
    }

    pub async fn run_after_agent(&self, messages: &mut Vec<Message>) -> Result<(), SynapticError> {
        for mw in self.middlewares.iter().rev() {
            mw.after_agent(messages).await?;
        }
        Ok(())
    }

    pub async fn run_before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
        for mw in &self.middlewares {
            mw.before_model(request).await?;
        }
        Ok(())
    }

    pub async fn run_after_model(
        &self,
        request: &ModelRequest,
        response: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        for mw in self.middlewares.iter().rev() {
            mw.after_model(request, response).await?;
        }
        Ok(())
    }

    /// Execute a model call through the full middleware chain.
    ///
    /// Runs the complete lifecycle: `before_model` -> `wrap_model_call`
    /// chain -> `after_model`.
    pub async fn call_model(
        &self,
        mut request: ModelRequest,
        base: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        // Run before_model hooks
        self.run_before_model(&mut request).await?;

        // Build the wrapped call chain (outermost first)
        let mut response = if self.middlewares.is_empty() {
            base.call(request.clone()).await?
        } else {
            let chain = WrapModelChain {
                middlewares: &self.middlewares,
                index: 0,
                base,
            };
            chain.call(request.clone()).await?
        };

        // Run after_model hooks
        self.run_after_model(&request, &mut response).await?;

        Ok(response)
    }

    /// Execute a tool call through the full middleware chain.
    pub async fn call_tool(
        &self,
        request: ToolCallRequest,
        base: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        if self.middlewares.is_empty() {
            base.call(request).await
        } else {
            let chain = WrapToolChain {
                middlewares: &self.middlewares,
                index: 0,
                base,
            };
            chain.call(request).await
        }
    }

    pub async fn run_before_file_op(&self, op: &FileOp) -> Result<FileOpDecision, SynapticError> {
        for mw in &self.middlewares {
            match mw.before_file_op(op).await? {
                FileOpDecision::Allow => continue,
                deny => return Ok(deny),
            }
        }
        Ok(FileOpDecision::Allow)
    }

    pub async fn run_after_file_op(
        &self,
        op: &FileOp,
        result: &FileOpResult,
    ) -> Result<(), SynapticError> {
        for mw in self.middlewares.iter().rev() {
            mw.after_file_op(op, result).await?;
        }
        Ok(())
    }

    pub async fn run_before_command(
        &self,
        cmd: &CommandOp,
    ) -> Result<CommandDecision, SynapticError> {
        for mw in &self.middlewares {
            match mw.before_command(cmd).await? {
                CommandDecision::Allow => continue,
                deny => return Ok(deny),
            }
        }
        Ok(CommandDecision::Allow)
    }

    pub async fn run_after_command(
        &self,
        cmd: &CommandOp,
        result: &CommandResult,
    ) -> Result<(), SynapticError> {
        for mw in self.middlewares.iter().rev() {
            mw.after_command(cmd, result).await?;
        }
        Ok(())
    }
}

// Internal chain helpers for recursive wrap_model_call / wrap_tool_call

#[allow(deprecated)]
struct WrapModelChain<'a> {
    middlewares: &'a [Arc<dyn AgentMiddleware>],
    index: usize,
    base: &'a dyn ModelCaller,
}

#[allow(deprecated)]
#[async_trait]
impl ModelCaller for WrapModelChain<'_> {
    async fn call(&self, request: ModelRequest) -> Result<ModelResponse, SynapticError> {
        if self.index >= self.middlewares.len() {
            self.base.call(request).await
        } else {
            let next = WrapModelChain {
                middlewares: self.middlewares,
                index: self.index + 1,
                base: self.base,
            };
            self.middlewares[self.index]
                .wrap_model_call(request, &next)
                .await
        }
    }
}

#[allow(deprecated)]
struct WrapToolChain<'a> {
    middlewares: &'a [Arc<dyn AgentMiddleware>],
    index: usize,
    base: &'a dyn ToolCaller,
}

#[allow(deprecated)]
#[async_trait]
impl ToolCaller for WrapToolChain<'_> {
    async fn call(&self, request: ToolCallRequest) -> Result<Value, SynapticError> {
        if self.index >= self.middlewares.len() {
            self.base.call(request).await
        } else {
            let next = WrapToolChain {
                middlewares: self.middlewares,
                index: self.index + 1,
                base: self.base,
            };
            self.middlewares[self.index]
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
    async fn call(&self, request: ModelRequest) -> Result<ModelResponse, SynapticError> {
        let chat_request = request.to_chat_request();
        let response = self.model.chat(chat_request).await?;
        Ok(response.into())
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingMiddleware {
        before_count: AtomicUsize,
        after_count: AtomicUsize,
    }

    impl CountingMiddleware {
        fn new() -> Self {
            Self {
                before_count: AtomicUsize::new(0),
                after_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl AgentMiddleware for CountingMiddleware {
        async fn before_model(&self, _request: &mut ModelRequest) -> Result<(), SynapticError> {
            self.before_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after_model(
            &self,
            _request: &ModelRequest,
            _response: &mut ModelResponse,
        ) -> Result<(), SynapticError> {
            self.after_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn middleware_chain_creation() {
        let mw: Arc<dyn AgentMiddleware> = Arc::new(CountingMiddleware::new());
        let chain = MiddlewareChain::new(vec![mw]);
        assert!(!chain.is_empty());
    }

    #[test]
    fn empty_middleware_chain() {
        let chain = MiddlewareChain::new(vec![]);
        assert!(chain.is_empty());
    }

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

    #[tokio::test]
    async fn file_hook_default_allows() {
        let mw: Arc<dyn AgentMiddleware> = Arc::new(CountingMiddleware::new());
        let chain = MiddlewareChain::new(vec![mw]);
        let op = FileOp {
            path: "/tmp/test".to_string(),
            kind: FileOpKind::Write,
        };
        let decision = chain.run_before_file_op(&op).await.unwrap();
        assert!(matches!(decision, FileOpDecision::Allow));
    }

    #[tokio::test]
    async fn command_hook_default_allows() {
        let mw: Arc<dyn AgentMiddleware> = Arc::new(CountingMiddleware::new());
        let chain = MiddlewareChain::new(vec![mw]);
        let cmd = CommandOp {
            command: "ls".to_string(),
            args: vec![],
            working_dir: None,
        };
        let decision = chain.run_before_command(&cmd).await.unwrap();
        assert!(matches!(decision, CommandDecision::Allow));
    }
}
