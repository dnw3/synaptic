# 中间件

中间件在定义明确的生命周期节点拦截和转换智能体行为。中间件不直接修改智能体逻辑，而是包裹在模型调用和工具调用外层，添加横切关注点，如速率限制、人工审批、摘要生成和上下文管理。本页介绍中间件抽象、生命周期钩子以及可用的中间件类别。

## Interceptor Trait

所有中间件实现 `Interceptor` trait，提供四个带有默认空实现的钩子：

```rust
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// 在每次模型调用前执行。可以修改请求。
    /// 按正序执行（先注册先调用）。
    async fn before_model(&self, _req: &mut ModelRequest) -> Result<(), SynapticError> {
        Ok(())
    }

    /// 在每次模型调用后执行。可以修改响应。
    /// 按逆序执行（后注册先调用）。
    async fn after_model(
        &self,
        _req: &ModelRequest,
        _resp: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        Ok(())
    }

    /// 包裹模型调用。重写以拦截或修改请求/响应。
    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        next.call(request).await
    }

    /// 包裹工具调用。重写以拦截或修改工具执行。
    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        next.call(request).await
    }
}
```

每个钩子都有默认实现，直接透传不做修改。中间件只需覆盖它需要的钩子即可。

## 生命周期

单次智能体轮次遵循以下顺序：

```text
loop {
  before_model（正序） ->  wrap_model_call（洋葱） ->  after_model（逆序）
  for each tool_call { wrap_tool_call（洋葱） }
}
```

1. **`before_model`** -- 在 LLM 请求之前调用。可以修改 `ModelRequest`（如注入上下文、调整系统提示词、裁剪历史记录）。按**正序**执行（MW1, MW2, MW3）。
2. **`wrap_model_call`** -- 以洋葱模式包裹实际的模型调用（MW1 包裹 MW2 包裹 MW3 包裹 LLM）。可以进行重试、添加降级方案、缓存，或完全替换调用。
3. **`after_model`** -- 在 LLM 响应之后调用。可以修改 `ModelResponse`（如记录用量、修复工具调用）。按**逆序**执行（MW3, MW2, MW1）。
4. **`wrap_tool_call`** -- 以相同的洋葱模式包裹每个工具调用。可以审批/拒绝、添加日志，或修改参数。

## InterceptorChain

多个拦截器组合成一个 `InterceptorChain`。链会自动按正确的生命周期顺序执行拦截器：

```rust
use synaptic::middleware::InterceptorChain;

let chain = InterceptorChain::new(vec![
    Arc::new(ToolCallLimitMiddleware::new(10)),
    Arc::new(HumanInTheLoopMiddleware::new(callback)),
    Arc::new(SummarizationMiddleware::new(model, 4000)),
]);
```

### 执行顺序

给定三个按顺序注册的拦截器（MW1, MW2, MW3）：

```text
MW1.before_model -> MW2.before_model -> MW3.before_model   （正序）
  MW1.wrap 包裹 MW2 包裹 MW3 包裹 LLM                      （洋葱）
MW3.after_model -> MW2.after_model -> MW1.after_model       （逆序）
```

这确保 `before_model` 钩子按注册顺序看到请求，洋葱包裹给最外层拦截器首/末控制权，`after_model` 钩子按逆序看到响应。

## 可用中间件

### ToolCallLimitMiddleware

限制每个智能体会话中工具调用的总次数。当达到上限时，后续的工具调用会返回错误而不执行。

- **使用场景**：防止智能体在无限循环中反复调用工具导致失控。
- **配置**：`ToolCallLimitMiddleware::new(max_calls)`

### ModelCallLimitMiddleware

限制每次运行的模型调用次数，防止无限制的 LLM 调用。

- **配置**：`ModelCallLimitMiddleware::new(max_calls)`

### HumanInTheLoopMiddleware

在工具调用执行前通过审批回调进行路由。回调接收工具名称和参数，并返回审批决定。

- **使用场景**：需要人工审核的高风险操作（数据库写入、外部 API 调用）。
- **配置**：`HumanInTheLoopMiddleware::new(callback)` 或 `.for_tools(vec!["dangerous_tool"])` 仅保护特定工具。

### SummarizationMiddleware

监控消息历史长度，当超过 token 阈值时对较早的消息进行摘要。用摘要替换较远的消息，同时保留最近的消息。

- **使用场景**：积累大量消息历史的长期运行智能体。
- **配置**：`SummarizationMiddleware::new(summarizer_model, token_threshold)`

### ContextEditingMiddleware

在每次模型调用前使用可配置策略转换消息历史：

- **`ContextStrategy::LastN(n)`** -- 仅保留最后 N 条消息（保留开头的系统消息）。
- **`ContextStrategy::StripToolCalls`** -- 移除工具调用/结果消息，仅保留人类和 AI 的内容消息。

### ToolRetryMiddleware

以指数退避重试失败的工具调用。

- **配置**：`ToolRetryMiddleware::new(max_retries)`

### ModelFallbackMiddleware

在主模型失败时提供降级模型。按顺序尝试备选模型，直到有一个成功。

### SecurityMiddleware

基于风险等级的工具执行控制，支持可配置的确认策略。

### SsrfGuardMiddleware

通过拒绝私有 IP 和云元数据端点的请求来拦截 SSRF 攻击。

### CircuitBreakerMiddleware

使用熔断器模式防止级联故障。跟踪失败次数，达到阈值时打开熔断器。

### TodoListMiddleware

在每次模型调用前向智能体上下文注入任务列表。

## 中间件与图特性的对比

中间件和图特性（检查点、中断）服务于不同的目的：

| 关注点 | 中间件 | 图 |
|---------|--------|-----|
| 工具审批 | HumanInTheLoopMiddleware | interrupt_before("tools") |
| 上下文管理 | ContextEditingMiddleware | 自定义节点逻辑 |
| 速率限制 | ToolCallLimitMiddleware | 不适用 |
| 状态持久化 | 不适用 | Checkpointer |

中间件在单个智能体节点内运行。图特性在整个图上运行。对于每轮次的关注点使用中间件，对于工作流级别的关注点使用图特性。

## 另请参阅

- [中间件使用指南](../how-to/middleware/index.md) -- 每种中间件的详细用法
- [工具调用限制](../how-to/middleware/tool-call-limit.md) -- 限制工具调用次数
- [人机协作](../how-to/middleware/human-in-the-loop.md) -- 审批工作流
- [摘要生成](../how-to/middleware/summarization.md) -- 自动上下文摘要
- [上下文编辑](../how-to/middleware/context-editing.md) -- 消息历史策略
