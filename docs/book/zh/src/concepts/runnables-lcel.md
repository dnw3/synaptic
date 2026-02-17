# Runnables 与 LCEL

LCEL（LangChain Expression Language）是一种声明式的组件组合方式。在 Synapse 中，LCEL 通过 `Runnable` trait 和管道运算符 `|` 实现，让你可以像搭积木一样将组件串联成链式调用。

## Runnable trait

`Runnable<I, O>` 是 Synapse 的核心组合抽象。任何可以接受输入并产生输出的组件都实现了这个 trait：

```rust
#[async_trait]
pub trait Runnable<I, O>: Send + Sync
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// 执行单次调用
    async fn invoke(&self, input: I, config: &RunnableConfig) -> Result<O, SynapseError>;

    /// 批量执行（默认实现为顺序调用 invoke）
    async fn batch(&self, inputs: Vec<I>, config: &RunnableConfig) -> Vec<Result<O, SynapseError>>;

    /// 流式输出（默认实现将 invoke 结果包装为单元素流）
    fn stream<'a>(&'a self, input: I, config: &'a RunnableConfig) -> RunnableOutputStream<'a, O>;

    /// 将自身包装为类型擦除的 BoxRunnable
    fn boxed(self) -> BoxRunnable<I, O>;
}
```

四个方法各有用途：

- **`invoke()`** -- 单次调用，输入一个值，返回一个结果。这是唯一必须实现的方法。
- **`batch()`** -- 批量调用，处理多个输入。默认实现为顺序执行 `invoke()`。
- **`stream()`** -- 流式调用，逐步产出结果。默认实现将 `invoke()` 的结果包装为单元素流，可以重写以实现真正的逐 token 流式输出。
- **`boxed()`** -- 将具体类型转换为类型擦除的 `BoxRunnable`，使其可以与 `|` 运算符组合。

## BoxRunnable 与管道运算符

`BoxRunnable` 是类型擦除的 `Runnable` 包装器。它的核心价值在于支持 `|` 管道运算符（通过 `BitOr` trait 实现），让你可以将多个步骤串联为一个链：

```rust
use synaptic_runnables::{BoxRunnable, Runnable};

// prompt 产出 Vec<Message>，model 接受并返回 String，parser 提取最终结果
let chain = prompt.boxed() | model.boxed() | parser.boxed();

// 一次调用执行整个链
let config = RunnableConfig::default();
let result = chain.invoke(input, &config).await?;
```

管道运算符将前一步的输出自动传递为后一步的输入。类型系统确保相邻步骤的输入输出类型匹配——如果不匹配，编译器会报错。

`BoxRunnable` 还提供了以下方法：

- **`stream()`** -- 流式执行链，最终组件的 `stream()` 提供真正的流式输出
- **`bind()`** -- 附加配置变换
- **`with_config()`** -- 固定使用指定配置
- **`with_listeners()`** -- 添加前后回调监听器

## 核心组合类型

Synapse 提供了丰富的 `Runnable` 实现类型，覆盖常见的组合模式：

### RunnablePassthrough

不修改输入，直接传递。常用于 `RunnableParallel` 中保留原始输入：

```rust
use synaptic_runnables::RunnablePassthrough;

let passthrough = RunnablePassthrough;
let result = passthrough.invoke("hello".to_string(), &config).await?;
assert_eq!(result, "hello");
```

### RunnableLambda

将异步闭包包装为 `Runnable`。这是最简单的创建自定义步骤的方式：

```rust
use synaptic_runnables::RunnableLambda;

let double = RunnableLambda::new(|x: i64| async move {
    Ok(x * 2)
});

let result = double.invoke(21, &config).await?; // 42
```

### RunnableSequence

将两个 `Runnable` 串联为一个序列。通常你不需要直接使用它——管道运算符 `|` 会自动创建 `RunnableSequence`。

### RunnableParallel

并行执行多个命名分支，将结果合并为 `serde_json::Value`（JSON 对象）：

```rust
use synaptic_runnables::RunnableParallel;

let parallel = RunnableParallel::new()
    .branch("translation", translator.boxed())
    .branch("summary", summarizer.boxed());

let result = parallel.invoke(input, &config).await?;
// result["translation"] 和 result["summary"] 分别包含各分支的结果
```

各分支使用 `tokio::join!` 并发执行，显著提升处理多个独立任务时的性能。

### RunnableBranch

根据条件路由到不同的分支。按顺序检查条件，第一个匹配的分支被执行。如果没有条件匹配，则执行默认分支：

```rust
use synaptic_runnables::RunnableBranch;

let branch = RunnableBranch::new(
    vec![
        (|input: &String| input.contains("math"), calculator.boxed()),
        (|input: &String| input.contains("weather"), weather_tool.boxed()),
    ],
    default_handler.boxed(),
);
```

### RunnableWithFallbacks

主链失败时自动尝试回退链。适合实现模型降级策略：

```rust
use synaptic_runnables::RunnableWithFallbacks;

let with_fallbacks = RunnableWithFallbacks::new(
    primary_model.boxed(),
    vec![fallback_model.boxed()],
);
// 如果 primary_model 失败，自动尝试 fallback_model
```

### RunnableAssign

将并行分支的结果合并到原始输入的 JSON 对象中。输入必须是 `serde_json::Value`：

```rust
use synaptic_runnables::{RunnableAssign, RunnableParallel};

// 在原始 JSON 输入的基础上，添加 "enriched" 字段
let assign = RunnableAssign::new(
    RunnableParallel::new()
        .branch("enriched", enricher.boxed())
);
```

### RunnablePick

从 JSON 值中提取指定的 key，丢弃其他字段：

```rust
use synaptic_runnables::RunnablePick;

let pick = RunnablePick::new(vec!["name".to_string(), "age".to_string()]);
// 从 { "name": "Alice", "age": 30, "email": "..." } 中只保留 name 和 age
```

### RunnableEach

对输入列表中的每个元素分别调用内部 `Runnable`，将结果收集为列表：

```rust
use synaptic_runnables::BoxRunnable;

let each = BoxRunnable::map_each(single_item_processor);
// Vec<I> -> Vec<O>，对每个元素调用 single_item_processor
```

### RunnableRetry

在失败时自动重试内部 `Runnable`，可配置重试策略：

```rust
use synaptic_runnables::{RunnableRetry, RetryPolicy};

let retry = RunnableRetry::new(
    flaky_step.boxed(),
    RetryPolicy::new(3), // 最多重试 3 次
);
```

### RunnableGenerator

将异步生成器函数包装为 `Runnable`，支持真正的流式输出。

## bind() 配置变换

`BoxRunnable::bind()` 用于附加配置转换。它返回一个新的 `BoxRunnable`，在每次调用前修改 `RunnableConfig`：

```rust
let configured = chain.bind(|mut config| {
    config.tags.push("production".to_string());
    config
});
```

这对于在运行时动态调整配置（如添加标签、设置元数据）非常有用，而不需要修改链本身的逻辑。

## 流式传输

LCEL 链支持流式处理。`stream()` 方法返回 `RunnableOutputStream`，它是一个异步 `Stream`，逐步产出结果：

```rust
use futures::StreamExt;

let mut stream = chain.stream(input, &config);

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(value) => print!("{}", value),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

在管道链中，流式传输的工作方式如下：

- 最终组件如果重写了 `stream()` 方法，会提供真正的逐 token 流式输出
- 中间组件使用默认的 `stream()` 实现（包装 `invoke()`）
- 流从链的最终组件向调用者传播

## 最佳实践

1. **使用 `boxed()` 进行类型擦除** -- 管道运算符 `|` 要求两侧都是 `BoxRunnable`。在构建链之前，对每个步骤调用 `.boxed()`。
2. **优先使用 `RunnableLambda` 包装简单转换** -- 比实现完整的 `Runnable` trait 更简洁。只有在需要自定义 `stream()` 或 `batch()` 行为时才实现完整的 trait。
3. **使用 `RunnableParallel` 提升并发性能** -- 多个独立操作（如同时调用多个 API）可以并行执行。
4. **使用 `RunnableWithFallbacks` 提高可靠性** -- 主模型失败时自动切换到备用模型，适合生产环境。
5. **使用 `bind()` 注入运行时配置** -- 而不是硬编码在链的构建逻辑中。
6. **使用 `RunnableAssign` + `RunnablePick` 操作 JSON 数据流** -- 这是 LCEL 中处理结构化数据的惯用模式。
