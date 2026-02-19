# Tool Registry

`ToolRegistry` 是一个线程安全的工具集合，`SerialToolExecutor` 通过注册表按名称分发工具调用。两者均由 `synaptic-tools` crate 提供。

## ToolRegistry

`ToolRegistry` 将工具存储在 `Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>` 中。它实现了 `Clone`，可以跨线程共享。

### 创建和注册工具

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use synaptic::tools::ToolRegistry;
use serde_json::{json, Value};

/// 将输入原样返回。
#[tool]
async fn echo(
    #[args] args: Value,
) -> Result<Value, SynapticError> {
    Ok(json!({"echo": args}))
}

let registry = ToolRegistry::new();
registry.register(echo())?;
```

如果注册两个同名工具，第二次注册会替换第一次。

### 查找工具

使用 `get()` 按名称检索工具：

```rust
let tool = registry.get("echo");
assert!(tool.is_some());

let missing = registry.get("nonexistent");
assert!(missing.is_none());
```

`get()` 返回 `Option<Arc<dyn Tool>>`，因此如果需要，可以直接调用该工具。

## SerialToolExecutor

`SerialToolExecutor` 包装了一个 `ToolRegistry`，并提供了一个便捷方法，可以在一步操作中按名称查找工具并调用它。

### 创建和使用

```rust
use synaptic::tools::SerialToolExecutor;
use serde_json::json;

let executor = SerialToolExecutor::new(registry);

let result = executor.execute("echo", json!({"message": "hello"})).await?;
assert_eq!(result, json!({"echo": {"message": "hello"}}));
```

`execute()` 方法：

1. 在注册表中按名称查找工具。
2. 使用提供的参数调用 `tool.call(args)`。
3. 返回结果，如果工具不存在则返回 `SynapticError::ToolNotFound`。

### 处理未知工具

如果使用未注册的名称调用 `execute()`，它会返回 `SynapticError::ToolNotFound`：

```rust
let err = executor.execute("nonexistent", json!({})).await.unwrap_err();
assert!(matches!(err, synaptic::core::SynapticError::ToolNotFound(name) if name == "nonexistent"));
```

## 完整示例

以下是一个注册多个工具并执行它们的完整示例：

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use synaptic::tools::{ToolRegistry, SerialToolExecutor};
use serde_json::{json, Value};

/// 两数相加。
#[tool]
async fn add(
    /// 第一个数
    a: f64,
    /// 第二个数
    b: f64,
) -> Result<Value, SynapticError> {
    Ok(json!({"result": a + b}))
}

/// 两数相乘。
#[tool]
async fn multiply(
    /// 第一个数
    a: f64,
    /// 第二个数
    b: f64,
) -> Result<Value, SynapticError> {
    Ok(json!({"result": a * b}))
}

#[tokio::main]
async fn main() -> Result<(), SynapticError> {
    let registry = ToolRegistry::new();
    registry.register(add())?;
    registry.register(multiply())?;

    let executor = SerialToolExecutor::new(registry);

    let sum = executor.execute("add", json!({"a": 3, "b": 4})).await?;
    assert_eq!(sum, json!({"result": 7.0}));

    let product = executor.execute("multiply", json!({"a": 3, "b": 4})).await?;
    assert_eq!(product, json!({"result": 12.0}));

    Ok(())
}
```

## 与 Chat Model 集成

在典型的 Agent 工作流中，模型的响应包含 `ToolCall` 条目。你需要通过执行器分发它们，并将结果发送回模型：

```rust
use synaptic::core::{Message, ToolCall};
use serde_json::json;

// After model responds with tool calls:
let tool_calls = vec![
    ToolCall {
        id: "call-1".to_string(),
        name: "add".to_string(),
        arguments: json!({"a": 3, "b": 4}),
    },
];

// Execute each tool call
for tc in &tool_calls {
    let result = executor.execute(&tc.name, tc.arguments.clone()).await?;

    // Create a tool message with the result
    let tool_message = Message::tool(
        result.to_string(),
        &tc.id,
    );
    // Append tool_message to the conversation and send back to the model
}
```

参见 [ReAct Agent 教程](../../tutorials/react-agent.md)获取完整的 Agent 循环示例。
