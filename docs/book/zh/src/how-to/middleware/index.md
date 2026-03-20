# Middleware 概述

Middleware 系统在 Agent 生命周期的每个节点拦截和修改行为——每次模型调用前/后，以及每次工具调用前后。当你需要处理横切关注点（速率限制、重试、上下文管理）而不修改 Agent 逻辑时，可以使用 Middleware。

## Interceptor Trait

所有方法都有默认的空实现。只需重写你需要的钩子方法即可。

```rust,ignore
#[async_trait]
pub trait Interceptor: Send + Sync {
    async fn before_model(&self, req: &mut ModelRequest) -> Result<(), SynapticError>;
    async fn after_model(&self, req: &ModelRequest, resp: &mut ModelResponse) -> Result<(), SynapticError>;
    async fn wrap_model_call(&self, request: ModelRequest, next: &dyn ModelCaller) -> Result<ModelResponse, SynapticError>;
    async fn wrap_tool_call(&self, request: ToolCallRequest, next: &dyn ToolCaller) -> Result<Value, SynapticError>;
}
```

## 生命周期图

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

内部循环在每个 Agent 步骤（模型调用后跟工具执行）中重复执行。`before_model` / `after_model` 在每次模型调用前后执行，可以修改请求或响应。`wrap_model_call` 和 `wrap_tool_call` 是洋葱式包装器，接收一个 `next` 调用器以委托给下一层。

## InterceptorChain

`InterceptorChain` 组合多个拦截器，对 `before_model` 钩子按注册顺序执行，对 `after_model` 钩子按反序执行。`wrap_model_call` 和 `wrap_tool_call` 钩子使用洋葱式嵌套。

```rust,ignore
use synaptic::middleware::InterceptorChain;

let chain = InterceptorChain::new(vec![
    Arc::new(ModelCallLimitMiddleware::new(10)),
    Arc::new(ToolRetryMiddleware::new(3)),
]);
```

## 在 `create_agent` 中使用 Middleware

通过 `AgentOptions::middleware` 传入拦截器。Agent 图会自动将它们连接到模型节点和工具节点。

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

## 文件与 Shell 钩子

Middleware 系统还提供了文件操作和 Shell 命令的钩子。这些钩子在 Deep Agent 工具执行文件系统或命令操作时被调用，允许你拦截并授权或拒绝操作。

```rust,ignore
use synaptic::middleware::{FileOp, FileOpDecision, CommandOp, CommandDecision};

struct MySecurityMiddleware;

#[async_trait]
impl Interceptor for MySecurityMiddleware {
    // ... 按需实现 model/tool 钩子 ...
}

// 文件/Shell 钩子通过 InterceptorChain 单独分发。
```

## 内置 Middleware

| Middleware | 使用的钩子 | 说明 |
|-----------|-----------|------|
| [`ModelCallLimitMiddleware`](model-call-limit.md) | `wrap_model_call` | 限制每次运行的模型调用次数 |
| [`ToolCallLimitMiddleware`](tool-call-limit.md) | `wrap_tool_call` | 限制每次运行的工具调用次数 |
| [`ToolRetryMiddleware`](tool-retry.md) | `wrap_tool_call` | 以指数退避重试失败的工具调用 |
| [`ModelFallbackMiddleware`](model-fallback.md) | `wrap_model_call` | 主模型失败时回退到备用模型 |
| [`SummarizationMiddleware`](summarization.md) | `before_model` | 上下文超过 token 限制时自动摘要 |
| [`TodoListMiddleware`](todo-list.md) | `before_model` | 向 Agent 上下文注入任务列表 |
| [`HumanInTheLoopMiddleware`](human-in-the-loop.md) | `wrap_tool_call` | 在工具执行前暂停以等待人工审批 |
| [`ContextEditingMiddleware`](context-editing.md) | `before_model` | 在模型调用前裁剪或过滤上下文 |
| [`SsrfGuardMiddleware`](ssrf-guard.md) | `wrap_tool_call` | 拦截 SSRF 攻击（私有 IP、元数据端点） |
| [`CircuitBreakerMiddleware`](circuit-breaker.md) | `wrap_tool_call` / `wrap_model_call` | 通过熔断器模式防止级联故障 |

## 编写自定义 Middleware

使用中间件宏可以快速定义自定义 Middleware，无需手动实现 `Interceptor` trait。每个宏对应一个钩子方法：

```rust,ignore
use synaptic::macros::before_model;
use synaptic::middleware::ModelRequest;
use synaptic::core::SynapticError;

// 使用 #[before_model] 宏——函数会自动生成 LoggingMiddleware 结构体和 Interceptor 实现
#[before_model]
async fn logging(request: &mut ModelRequest) -> Result<(), SynapticError> {
    println!("模型调用，包含 {} 条消息", request.messages.len());
    Ok(())
}
```

然后将其添加到 Agent 中：

```rust,ignore
let options = AgentOptions {
    middleware: vec![logging()],  // logging() 返回 Arc<dyn Interceptor>
    ..Default::default()
};
let graph = create_agent(model, tools, options)?;
```

> **提示：** 除了 `#[before_model]`，还有 `#[after_model]`、`#[wrap_model_call]`、`#[wrap_tool_call]`、`#[system_prompt]` 等宏，分别对应不同的钩子。详见[过程宏](../macros.md)。如果需要更精细的控制，也可以手动实现 `Interceptor` trait。
