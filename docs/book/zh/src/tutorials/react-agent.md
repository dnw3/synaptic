# 构建 ReAct Agent

本教程将引导你构建一个 ReAct（Reasoning + Acting）Agent。ReAct Agent 能够推理问题、调用工具获取信息，然后基于工具返回的结果做出最终回答。这是构建智能 AI Agent 的核心模式。

## 你将学到什么

- 实现自定义 `Tool` trait
- 使用 `create_react_agent` 创建 ReAct Agent
- 理解 ReAct 循环的工作原理

## ReAct 循环

ReAct Agent 遵循一个简单的循环：

```text
用户提问 → LLM 决策 → 工具执行 → LLM 审查 → 重复或完成
```

1. **LLM 决策** -- 模型分析问题，决定是否需要调用工具。如果需要，返回 `ToolCall`。
2. **工具执行** -- `ToolNode` 自动执行被调用的工具，将结果作为 `Tool` 消息返回。
3. **LLM 审查** -- 模型查看工具执行结果，决定是否需要更多工具调用，还是可以给出最终答案。
4. **重复或完成** -- 如果需要更多信息，循环继续；否则返回最终回答。

## 完整代码

```rust
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;
use synaptic_core::{ChatModel, ChatRequest, ChatResponse, Message, SynapseError, Tool, ToolCall};
use synaptic_graph::{create_react_agent, MessageState};

// 自定义模型（演示用，模拟 LLM 的工具调用行为）
struct DemoModel;

#[async_trait]
impl ChatModel for DemoModel {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError> {
        // 检查是否已经有工具调用的结果
        let has_tool_output = request.messages.iter().any(|m| m.is_tool());

        if !has_tool_output {
            // 第一次调用：决定使用工具
            Ok(ChatResponse {
                message: Message::ai_with_tool_calls(
                    "I will use a tool to calculate this.",
                    vec![ToolCall {
                        id: "call-1".to_string(),
                        name: "add".to_string(),
                        arguments: json!({ "a": 7, "b": 5 }),
                    }],
                ),
                usage: None,
            })
        } else {
            // 第二次调用：已有工具结果，给出最终回答
            Ok(ChatResponse {
                message: Message::ai("The result is 12."),
                usage: None,
            })
        }
    }
}

// 加法工具
struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &'static str {
        "add"
    }

    fn description(&self) -> &'static str {
        "Adds two numbers."
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, SynapseError> {
        let a = args["a"].as_i64().unwrap_or_default();
        let b = args["b"].as_i64().unwrap_or_default();
        Ok(json!({ "value": a + b }))
    }
}

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    // 1. 创建模型和工具
    let model = Arc::new(DemoModel);
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(AddTool)];

    // 2. 创建 ReAct Agent
    let graph = create_react_agent(model, tools)?;

    // 3. 构建初始状态
    let state = MessageState {
        messages: vec![Message::human("What is 7 + 5?")],
    };

    // 4. 执行 Agent
    let result = graph.invoke(state).await?;

    // 5. 获取最终回答
    println!("answer: {}", result.last_message().unwrap().content());

    Ok(())
}
```

运行后输出：

```
answer: The result is 12.
```

## 逐步解析

### 1. 实现 Tool trait

```rust
struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &'static str { "add" }
    fn description(&self) -> &'static str { "Adds two numbers." }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, SynapseError> {
        let a = args["a"].as_i64().unwrap_or_default();
        let b = args["b"].as_i64().unwrap_or_default();
        Ok(json!({ "value": a + b }))
    }
}
```

每个工具需要实现三个方法：
- `name()` -- 工具名称，LLM 通过此名称调用工具
- `description()` -- 工具描述，帮助 LLM 理解何时使用此工具
- `call()` -- 实际执行逻辑，接收 JSON 参数，返回 JSON 结果

### 2. 创建 ReAct Agent

```rust
let model = Arc::new(DemoModel);
let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(AddTool)];
let graph = create_react_agent(model, tools)?;
```

`create_react_agent` 是 Synapse 提供的便捷函数，等价于 LangChain 的同名函数。它内部构建了一个 `StateGraph`，包含：
- **agent 节点** -- 调用 LLM 进行推理
- **tools 节点** -- 使用 `ToolNode` 执行工具
- **条件边** -- 根据 LLM 是否返回了 `ToolCall` 决定下一步走向

### 3. 执行 Agent

```rust
let state = MessageState {
    messages: vec![Message::human("What is 7 + 5?")],
};
let result = graph.invoke(state).await?;
```

`MessageState` 包含消息列表，是 ReAct Agent 的标准状态类型。调用 `invoke()` 后，Agent 会自动执行 ReAct 循环，直到 LLM 给出不包含工具调用的最终回答。

### 4. 执行流程

以本示例为例，完整的执行流程如下：

```text
1. [agent 节点] 收到 "What is 7 + 5?"
   → LLM 返回 ToolCall { name: "add", arguments: { a: 7, b: 5 } }

2. [条件边] 检测到 ToolCall → 转到 tools 节点

3. [tools 节点] 执行 AddTool::call({ a: 7, b: 5 })
   → 返回 Tool 消息 { value: 12 }

4. [agent 节点] 收到工具结果
   → LLM 返回 "The result is 12."（无 ToolCall）

5. [条件边] 未检测到 ToolCall → 结束
```

## 使用真实 LLM

在实际应用中，将 `DemoModel` 替换为真实的模型适配器：

```rust
use synaptic_models::OpenAiChatModel;

let model = Arc::new(OpenAiChatModel::new("gpt-4o"));
```

真实的 LLM 会自动根据对话内容和工具描述决定是否调用工具。你只需要实现好工具的 `call()` 方法，其余的推理过程由 LLM 完成。

## 添加更多工具

只需将更多工具加入列表即可：

```rust
let tools: Vec<Arc<dyn Tool>> = vec![
    Arc::new(AddTool),
    Arc::new(SearchTool),
    Arc::new(WeatherTool),
];
let graph = create_react_agent(model, tools)?;
```

LLM 会根据问题自动选择合适的工具。

## 下一步

- [构建 Graph 工作流](graph-workflow.md) -- 构建自定义状态机工作流
- [Graph 概念](../concepts/graph.md) -- 深入了解 StateGraph 的工作原理
- [什么是 Synapse？](../what-is-synapse.md) -- 回顾 LangChain 到 Synapse 的概念映射
