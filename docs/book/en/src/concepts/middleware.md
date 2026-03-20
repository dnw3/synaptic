# Middleware

Middleware intercepts and transforms agent behavior at well-defined lifecycle points. Rather than modifying agent logic directly, middleware wraps around model calls and tool calls, adding cross-cutting concerns like rate limiting, human approval, summarization, and context management. This page explains the middleware abstraction, the lifecycle hooks, and the available middleware classes.

## The Interceptor Trait

All middleware implements the `Interceptor` trait, which provides four hooks with no-op defaults:

```rust
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// Called before each model invocation. Can modify the request.
    /// Runs in forward order (first added -> first called).
    async fn before_model(&self, _req: &mut ModelRequest) -> Result<(), SynapticError> {
        Ok(())
    }

    /// Called after each model invocation. Can modify the response.
    /// Runs in reverse order (last added -> first called).
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
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        next.call(request).await
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
```

Each hook has a default implementation that passes through unchanged. Middleware only overrides the hooks it needs.

## Lifecycle

A single agent turn follows this sequence:

```text
loop {
  before_model (forward)  ->  wrap_model_call (onion)  ->  after_model (reverse)
  for each tool_call { wrap_tool_call (onion) }
}
```

1. **`before_model`** -- called before each LLM request. Can modify the `ModelRequest` (e.g., inject context, tweak system prompt, trim history). Runs in **forward** order (MW1, MW2, MW3).
2. **`wrap_model_call`** -- wraps the actual model invocation in an onion pattern (MW1 wraps MW2 wraps MW3 wraps LLM). Can retry, add fallbacks, cache, or replace the call entirely.
3. **`after_model`** -- called after the LLM responds. Can modify the `ModelResponse` (e.g., log usage, fix tool calls). Runs in **reverse** order (MW3, MW2, MW1).
4. **`wrap_tool_call`** -- wraps each tool invocation in the same onion pattern. Can approve/reject, add logging, or modify arguments.

## InterceptorChain

Multiple interceptors are composed into an `InterceptorChain`. The chain applies interceptors in the correct lifecycle order automatically:

```rust
use synaptic::middleware::InterceptorChain;

let chain = InterceptorChain::new(vec![
    Arc::new(ToolCallLimitMiddleware::new(10)),
    Arc::new(HumanInTheLoopMiddleware::new(callback)),
    Arc::new(SummarizationMiddleware::new(model, 4000)),
]);
```

### Execution Order

Given three interceptors (MW1, MW2, MW3) registered in order:

```text
MW1.before_model -> MW2.before_model -> MW3.before_model   (forward)
  MW1.wrap wraps MW2 wraps MW3 wraps LLM                   (onion)
MW3.after_model -> MW2.after_model -> MW1.after_model       (reverse)
```

This ensures `before_model` hooks see the request in registration order, the onion wrapping gives the outermost interceptor first/last control, and `after_model` hooks see the response in reverse order.

## Available Middleware

### ToolCallLimitMiddleware

Limits the total number of tool calls per agent session. When the limit is reached, subsequent tool calls return an error instead of executing.

- **Use case**: Preventing runaway agents that call tools in an infinite loop.
- **Configuration**: `ToolCallLimitMiddleware::new(max_calls)`

### ModelCallLimitMiddleware

Limits model invocations per run, preventing unbounded LLM calls.

- **Configuration**: `ModelCallLimitMiddleware::new(max_calls)`

### HumanInTheLoopMiddleware

Routes tool calls through an approval callback before execution. The callback receives the tool name and arguments and returns an approval decision.

- **Use case**: High-stakes operations (database writes, external API calls) that require human review.
- **Configuration**: `HumanInTheLoopMiddleware::new(callback)` or `.for_tools(vec!["dangerous_tool"])` to guard only specific tools.

### SummarizationMiddleware

Monitors message history length and summarizes older messages when a token threshold is exceeded. Replaces distant messages with a summary while preserving recent ones.

- **Use case**: Long-running agents that accumulate large message histories.
- **Configuration**: `SummarizationMiddleware::new(summarizer_model, token_threshold)`

### ContextEditingMiddleware

Transforms the message history before each model call using a configurable strategy:

- **`ContextStrategy::LastN(n)`** -- keep only the last N messages (preserving leading system messages).
- **`ContextStrategy::StripToolCalls`** -- remove tool call/result messages, keeping only human and AI content messages.

### ToolRetryMiddleware

Retries failed tool calls with exponential backoff.

- **Configuration**: `ToolRetryMiddleware::new(max_retries)`

### ModelFallbackMiddleware

Provides fallback models when the primary model fails. Tries alternatives in order until one succeeds.

### SecurityMiddleware

Risk-based tool execution gating with configurable confirmation policies.

### SsrfGuardMiddleware

Blocks SSRF attacks by denying requests to private IPs and cloud metadata endpoints.

### CircuitBreakerMiddleware

Prevents cascading failures using the circuit breaker pattern. Tracks failures and opens the circuit when a threshold is reached.

### TodoListMiddleware

Injects a task list into the agent context before each model call.

## Middleware vs. Graph Features

Middleware and graph features (checkpointing, interrupts) serve different purposes:

| Concern | Middleware | Graph |
|---------|-----------|-------|
| Tool approval | HumanInTheLoopMiddleware | interrupt_before("tools") |
| Context management | ContextEditingMiddleware | Custom node logic |
| Rate limiting | ToolCallLimitMiddleware | Not applicable |
| State persistence | Not applicable | Checkpointer |

Middleware operates within a single agent node. Graph features operate across the entire graph. Use middleware for per-turn concerns and graph features for workflow-level concerns.

## See Also

- [Middleware How-to Guides](../how-to/middleware/index.md) -- detailed usage for each middleware class
- [Tool Call Limit](../how-to/middleware/tool-call-limit.md) -- limiting tool calls
- [Human-in-the-Loop](../how-to/middleware/human-in-the-loop.md) -- approval workflows
- [Summarization](../how-to/middleware/summarization.md) -- automatic context summarization
- [Context Editing](../how-to/middleware/context-editing.md) -- message history strategies
