# Middleware Overview

The middleware system intercepts and modifies agent behavior at every lifecycle point -- before/after each model call, and around each tool call. Use middleware when you need cross-cutting concerns (rate limiting, retries, context management) without modifying your agent logic.

## Interceptor Trait

All methods have default no-op implementations. Override only the hooks you need.

```rust,ignore
#[async_trait]
pub trait Interceptor: Send + Sync {
    async fn before_model(&self, req: &mut ModelRequest) -> Result<(), SynapticError>;
    async fn after_model(&self, req: &ModelRequest, resp: &mut ModelResponse) -> Result<(), SynapticError>;
    async fn wrap_model_call(&self, request: ModelRequest, next: &dyn ModelCaller) -> Result<ModelResponse, SynapticError>;
    async fn wrap_tool_call(&self, request: ToolCallRequest, next: &dyn ToolCaller) -> Result<Value, SynapticError>;
}
```

## Lifecycle Diagram

```text
loop {
  before_model(request)
    -> wrap_model_call(request, next)
  after_model(request, response)
  for each tool_call {
    wrap_tool_call(request, next)
  }
}
```

The inner loop repeats for each agent step (model call followed by tool execution). `before_model` / `after_model` run around every model call and can mutate the request or response. `wrap_model_call` and `wrap_tool_call` are onion-style wrappers that receive a `next` caller to delegate to the next layer.

## InterceptorChain

`InterceptorChain` composes multiple interceptors and executes them in registration order for `before_model`, and in reverse order for `after_model`. The `wrap_model_call` and `wrap_tool_call` hooks use onion-style nesting.

```rust,ignore
use synaptic::middleware::InterceptorChain;

let chain = InterceptorChain::new(vec![
    Arc::new(ModelCallLimitMiddleware::new(10)),
    Arc::new(ToolRetryMiddleware::new(3)),
]);
```

## Using Middleware with `create_agent`

Pass interceptors through `AgentOptions::middleware`. The agent graph wires them into both the model node and the tool node automatically.

```rust,ignore
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{ModelCallLimitMiddleware, ToolRetryMiddleware};

let options = AgentOptions {
    middleware: vec![
        Arc::new(ModelCallLimitMiddleware::new(10)),
        Arc::new(ToolRetryMiddleware::new(3)),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

## File & Shell Hooks

The middleware system also provides hooks for file operations and shell commands. These are called by Deep Agent tools when performing filesystem or command operations, allowing you to intercept and authorize or deny actions.

```rust,ignore
use synaptic::middleware::{FileOp, FileOpDecision, CommandOp, CommandDecision};

struct MySecurityMiddleware;

#[async_trait]
impl Interceptor for MySecurityMiddleware {
    // ... model/tool hooks as needed ...
}

// File/shell hooks are dispatched separately through the InterceptorChain.
```

## Built-in Middlewares

| Middleware | Hook Used | Description |
|-----------|-----------|-------------|
| [`ModelCallLimitMiddleware`](model-call-limit.md) | `wrap_model_call` | Limits model invocations per run |
| [`ToolCallLimitMiddleware`](tool-call-limit.md) | `wrap_tool_call` | Limits tool invocations per run |
| [`ToolRetryMiddleware`](tool-retry.md) | `wrap_tool_call` | Retries failed tools with exponential backoff |
| [`ModelFallbackMiddleware`](model-fallback.md) | `wrap_model_call` | Falls back to alternative models on failure |
| [`SummarizationMiddleware`](summarization.md) | `before_model` | Auto-summarizes when context exceeds token limit |
| [`TodoListMiddleware`](todo-list.md) | `before_model` | Injects a task list into the agent context |
| [`HumanInTheLoopMiddleware`](human-in-the-loop.md) | `wrap_tool_call` | Pauses for human approval before tool execution |
| [`ContextEditingMiddleware`](context-editing.md) | `before_model` | Trims or filters context before model calls |
| [`SsrfGuardMiddleware`](ssrf-guard.md) | `wrap_tool_call` | Blocks SSRF attacks (private IPs, metadata endpoints) |
| [`CircuitBreakerMiddleware`](circuit-breaker.md) | `wrap_tool_call` / `wrap_model_call` | Prevents cascading failures with circuit breaker pattern |

## Writing a Custom Middleware

The easiest way to define a middleware is with the corresponding macro. Each lifecycle hook has its own macro (`#[before_model]`, `#[after_model]`, `#[wrap_model_call]`, `#[wrap_tool_call]`, `#[system_prompt]`). The macro generates the struct, `Interceptor` trait implementation, and a factory function automatically.

```rust,ignore
use synaptic::macros::before_model;
use synaptic::middleware::ModelRequest;
use synaptic::core::SynapticError;

#[before_model]
async fn log_model_call(request: &mut ModelRequest) -> Result<(), SynapticError> {
    println!("Model call with {} messages", request.messages.len());
    Ok(())
}
```

Then add it to your agent:

```rust,ignore
let options = AgentOptions {
    middleware: vec![log_model_call()],
    ..Default::default()
};
let graph = create_agent(model, tools, options)?;
```

> **Note:** The `log_model_call()` factory function returns `Arc<dyn Interceptor>`. For stateful middleware, use `#[field]` parameters on the function. See [Procedural Macros](../macros.md#middleware-macros) for the full reference, including all five middleware macros and stateful middleware with `#[field]`.
