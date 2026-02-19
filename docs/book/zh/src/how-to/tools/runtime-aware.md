# Runtime-Aware Tools

`RuntimeAwareTool` 扩展了基本的 `Tool` trait，增加了运行时上下文——当前图状态、Store 引用、流写入器、工具调用 ID 和 Runnable 配置。推荐使用 `#[tool]` 宏配合 `#[inject(...)]` 参数来定义此类工具。

## `ToolRuntime` 结构体

当 Runtime-Aware Tool 被调用时，它会收到一个包含以下字段的 `ToolRuntime`：

```rust,ignore
pub struct ToolRuntime {
    pub store: Option<Arc<dyn Store>>,
    pub stream_writer: Option<StreamWriter>,
    pub state: Option<Value>,
    pub tool_call_id: String,
    pub config: Option<RunnableConfig>,
}
```

| 字段 | 描述 |
|------|------|
| `store` | 共享的键值存储，用于跨工具持久化 |
| `stream_writer` | 用于在工具内部推送流式输出的写入器 |
| `state` | 当前图状态的序列化快照 |
| `tool_call_id` | 正在执行的工具调用的 ID |
| `config` | 包含标签、元数据和运行 ID 的 Runnable 配置 |

## 使用 `#[tool]` 宏定义 Runtime-Aware Tool

推荐使用 `#[tool]` 宏配合 `#[inject(...)]` 参数来定义 Runtime-Aware Tool。当函数中存在 `#[inject]` 参数时，宏会自动生成 `RuntimeAwareTool` 实现而非普通的 `Tool`。

支持三种注入类型：

| 注解 | 来源 | 典型类型 |
|------|------|---------|
| `#[inject(state)]` | `ToolRuntime::state`（反序列化为指定类型） | 自定义状态结构体或 `Value` |
| `#[inject(store)]` | `ToolRuntime::store`（`Arc<dyn Store>`） | `Arc<dyn Store>` |
| `#[inject(tool_call_id)]` | `ToolRuntime::tool_call_id` | `String` |

```rust,ignore
use std::sync::Arc;
use synaptic::macros::tool;
use synaptic::core::{Store, SynapticError};
use serde_json::{json, Value};

/// 将笔记保存到 Store 中。
#[tool]
async fn save_note(
    /// 笔记的键
    key: String,
    /// 笔记内容
    text: String,
    /// 注入：共享的键值存储
    #[inject(store)]
    store: Arc<dyn Store>,
) -> Result<Value, SynapticError> {
    store.put(
        "notes",
        &key,
        json!({"text": text}),
    ).await?;

    Ok(json!({"saved": key}))
}

// 工厂函数返回 Arc<dyn RuntimeAwareTool>
let tool = save_note();
assert_eq!(tool.name(), "save_note");
```

LLM 只能看到 `key` 和 `text` 参数，`store` 由 Agent 运行时自动注入。

## 在图中与 `ToolNode` 配合使用

`ToolNode` 会自动将运行时上下文注入到已注册的 `RuntimeAwareTool` 实例中。使用 `with_runtime_tool()` 注册它们，并可选择性地通过 `with_store()` 附加 Store：

```rust,ignore
use std::sync::Arc;
use synaptic::graph::ToolNode;
use synaptic::tools::{ToolRegistry, SerialToolExecutor};

let registry = ToolRegistry::new();
let executor = SerialToolExecutor::new(registry);

// save_note() 返回 Arc<dyn RuntimeAwareTool>
let tool_node = ToolNode::new(executor)
    .with_store(store.clone())
    .with_runtime_tool(save_note());
```

当图执行此工具节点并遇到匹配 `"save_note"` 的工具调用时，它会构建一个填充了当前图状态、Store 和工具调用 ID 的 `ToolRuntime`，然后调用 `call_with_runtime()`。

## `RuntimeAwareToolAdapter` -- 在图外使用

如果你需要在期望标准 `Tool` trait 的上下文中使用 `RuntimeAwareTool`（例如，直接与 `SerialToolExecutor` 配合使用），请用 `RuntimeAwareToolAdapter` 包装它：

```rust,ignore
use std::sync::Arc;
use synaptic::core::{RuntimeAwareTool, RuntimeAwareToolAdapter, ToolRuntime};

let tool = save_note(); // Arc<dyn RuntimeAwareTool>
let adapter = RuntimeAwareToolAdapter::new(tool);

// Optionally inject a runtime before calling
adapter.set_runtime(ToolRuntime {
    store: Some(store.clone()),
    stream_writer: None,
    state: None,
    tool_call_id: "call-1".to_string(),
    config: None,
}).await;

// Now use it as a regular Tool
let result = adapter.call(json!({"key": "k", "text": "hello"})).await?;
```

如果在调用 `call()` 之前未调用 `set_runtime()`，适配器会使用默认的空 `ToolRuntime`，所有可选字段设为 `None`，`tool_call_id` 为空字符串。

## 使用 Store 创建 `create_react_agent`

通过 `create_react_agent` 构建 ReAct Agent 时，可以通过 `AgentOptions` 传入 Store，它会自动连接到 `ToolNode`，供所有已注册的 Runtime-Aware Tool 使用：

```rust,ignore
use synaptic::graph::{create_react_agent, AgentOptions};

let graph = create_react_agent(
    model,
    tools,
    AgentOptions {
        store: Some(store),
        ..Default::default()
    },
);
```
