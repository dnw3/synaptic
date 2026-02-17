# Build a ReAct Agent

This tutorial walks you through building a ReAct (Reasoning + Acting) agent that can decide when to call tools and when to respond to the user. You will define a custom tool, wire it into a prebuilt agent graph, and watch the agent loop through reasoning and tool execution.

## What is a ReAct Agent?

A ReAct agent follows a loop:

1. **Reason** -- The LLM looks at the conversation so far and decides what to do next.
2. **Act** -- If the LLM determines it needs information, it emits one or more tool calls.
3. **Observe** -- The tool results are added to the conversation as `Tool` messages.
4. **Repeat** -- The LLM reviews the tool output and either calls more tools or produces a final answer.

Synapse provides `create_react_agent(model, tools)`, which builds a compiled `StateGraph` that implements this loop automatically.

## Prerequisites

Add the required crates to your `Cargo.toml`:

```toml
[dependencies]
synaptic-core = { path = "../crates/synaptic-core" }
synaptic-graph = { path = "../crates/synaptic-graph" }
synaptic-tools = { path = "../crates/synaptic-tools" }
async-trait = "0.1"
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Step 1: Define a Custom Tool

Every tool in Synapse implements the `Tool` trait. A tool has a name, a description (used by the LLM to decide when to call it), and an async `call` method that receives JSON arguments and returns a JSON result.

```rust
use async_trait::async_trait;
use serde_json::json;
use synaptic_core::{SynapseError, Tool};

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
```

The `args` parameter is a `serde_json::Value` -- typically a JSON object whose keys match whatever the LLM was told to provide. In production, you would validate the arguments more carefully and return a descriptive `SynapseError` on failure.

## Step 2: Create a Chat Model

For this tutorial we build a simple demo model that simulates the ReAct loop. On the first call (when there is no tool output in the conversation yet), it returns a tool call. On the second call (after tool output has been added), it returns a final text answer.

```rust
use async_trait::async_trait;
use serde_json::json;
use synaptic_core::{ChatModel, ChatRequest, ChatResponse, Message, SynapseError, ToolCall};

struct DemoModel;

#[async_trait]
impl ChatModel for DemoModel {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError> {
        let has_tool_output = request.messages.iter().any(|m| m.is_tool());

        if !has_tool_output {
            // First turn: ask to call the "add" tool
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
            // Second turn: the tool result is in, produce the final answer
            Ok(ChatResponse {
                message: Message::ai("The result is 12."),
                usage: None,
            })
        }
    }
}
```

In a real application you would use one of the provider adapters (`OpenAiChatModel`, `AnthropicChatModel`, etc.) instead of a scripted model.

## Step 3: Build the Agent Graph

`create_react_agent` takes a model and a vector of tools, and returns a `CompiledGraph<MessageState>`. Under the hood, it creates two nodes:

- **"agent"** -- calls the `ChatModel` with the current messages and tool definitions.
- **"tools"** -- executes any tool calls from the agent's response using a `ToolNode`.

A conditional edge routes from "agent" to "tools" if the response contains tool calls, or to `END` if it does not. An unconditional edge routes from "tools" back to "agent" so the model can review the results.

```rust
use std::sync::Arc;
use synaptic_core::Tool;
use synaptic_graph::create_react_agent;

let model = Arc::new(DemoModel);
let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(AddTool)];

let graph = create_react_agent(model, tools).unwrap();
```

Both the model and tools are wrapped in `Arc` because the graph needs shared ownership -- nodes may be invoked concurrently in more complex workflows.

## Step 4: Run the Agent

Create an initial `MessageState` with the user's question and invoke the graph:

```rust
use synaptic_core::Message;
use synaptic_graph::MessageState;

let initial_state = MessageState {
    messages: vec![Message::human("What is 7 + 5?")],
};

let result = graph.invoke(initial_state).await.unwrap();

let last = result.last_message().unwrap();
println!("agent answer: {}", last.content());
// Output: agent answer: The result is 12.
```

`MessageState` is the built-in state type for conversational agents. It holds a `Vec<Message>` that grows as the agent loop progresses. After invocation, `last_message()` returns the final message in the conversation -- typically the agent's answer.

## Full Working Example

Here is the complete program that ties all the pieces together:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;
use synaptic_core::{ChatModel, ChatRequest, ChatResponse, Message, SynapseError, Tool, ToolCall};
use synaptic_graph::{create_react_agent, MessageState};

// --- Model ---

struct DemoModel;

#[async_trait]
impl ChatModel for DemoModel {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError> {
        let has_tool_output = request.messages.iter().any(|m| m.is_tool());
        if !has_tool_output {
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
            Ok(ChatResponse {
                message: Message::ai("The result is 12."),
                usage: None,
            })
        }
    }
}

// --- Tool ---

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

// --- Main ---

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    let model = Arc::new(DemoModel);
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(AddTool)];

    let graph = create_react_agent(model, tools)?;

    let initial_state = MessageState {
        messages: vec![Message::human("What is 7 + 5?")],
    };

    let result = graph.invoke(initial_state).await?;
    let last = result.last_message().unwrap();
    println!("agent answer: {}", last.content());
    Ok(())
}
```

## How the Loop Executes

Here is the sequence of events when you run this example:

| Step | Node | What happens |
|------|------|-------------|
| 1 | **agent** | Receives `[Human("What is 7 + 5?")]`. Returns an AI message with a `ToolCall` for `add(a=7, b=5)`. |
| 2 | *routing* | The conditional edge sees tool calls in the last message and routes to **tools**. |
| 3 | **tools** | `ToolNode` looks up `"add"` in the registry, calls `AddTool::call`, and appends a `Tool` message with `{"value": 12}`. |
| 4 | *edge* | The unconditional edge routes from **tools** back to **agent**. |
| 5 | **agent** | Receives the full conversation including the tool result. Returns `AI("The result is 12.")` with no tool calls. |
| 6 | *routing* | No tool calls in the last message, so the conditional edge routes to `END`. |

The graph terminates and returns the final `MessageState`.

## Next Steps

- [Build a Graph Workflow](graph-workflow.md) -- build custom state graphs with conditional edges
- [Tool Choice](../how-to/tools/tool-choice.md) -- control which tools the model can call
- [Human-in-the-Loop](../how-to/graph/human-in-the-loop.md) -- add interrupt points for human review
- [Checkpointing](../how-to/graph/checkpointing.md) -- persist agent state across invocations
