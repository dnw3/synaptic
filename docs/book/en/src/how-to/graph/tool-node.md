# Tool Node

`ToolNode` is a prebuilt graph node that automatically dispatches tool calls found in the last AI message of the state. It bridges the `synaptic_tools` crate's execution infrastructure with the graph system, making it straightforward to build tool-calling agent loops.

## How It Works

When `ToolNode` processes state, it:

1. Reads the **last message** from the state.
2. Extracts any `tool_calls` from that message (AI messages carry tool call requests).
3. Executes each tool call through the provided `SerialToolExecutor`.
4. Appends a `Message::tool(result, call_id)` for each tool call result.
5. Returns the updated state.

If the last message has no tool calls, the node passes the state through unchanged.

## Setup

Create a `ToolNode` by providing a `SerialToolExecutor` with registered tools:

```rust
use synaptic_graph::ToolNode;
use synaptic_tools::{ToolRegistry, SerialToolExecutor};
use synaptic_core::{Tool, ToolDefinition, SynapseError};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// Define a tool
struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "calculator".to_string(),
            description: "Evaluates math expressions".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string" }
                },
                "required": ["expression"]
            }),
        }
    }

    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        let expr = args["expression"].as_str().unwrap_or("0");
        Ok(Value::String(format!("Result: {expr}")))
    }
}

// Register and create the executor
let registry = ToolRegistry::new();
registry.register(Arc::new(CalculatorTool)).await?;

let executor = SerialToolExecutor::new(registry);
let tool_node = ToolNode::new(executor);
```

## Using ToolNode in a Graph

`ToolNode` implements `Node<MessageState>`, so it can be added directly to a `StateGraph`:

```rust
use synaptic_graph::{StateGraph, FnNode, MessageState, END};
use synaptic_core::{Message, ToolCall};

// An agent node that produces tool calls
let agent = FnNode::new(|mut state: MessageState| async move {
    let tool_call = ToolCall {
        id: "call-1".to_string(),
        name: "calculator".to_string(),
        arguments: serde_json::json!({"expression": "2+2"}),
    };
    state.messages.push(Message::ai_with_tool_calls("", vec![tool_call]));
    Ok(state)
});

let graph = StateGraph::new()
    .add_node("agent", agent)
    .add_node("tools", tool_node)
    .set_entry_point("agent")
    .add_edge("agent", "tools")
    .add_edge("tools", END)
    .compile()?;

let result = graph.invoke(MessageState::new()).await?;
// State now contains:
//   [0] AI message with tool_calls
//   [1] Tool message with "Result: 2+2"
```

## Agent Loop Pattern

In a typical ReAct agent, the tool node feeds results back to the agent node, which decides whether to call more tools or produce a final answer. Use conditional edges to implement this loop:

```rust
use std::collections::HashMap;
use synaptic_graph::{StateGraph, MessageState, END};

let graph = StateGraph::new()
    .add_node("agent", agent_node)
    .add_node("tools", tool_node)
    .set_entry_point("agent")
    .add_conditional_edges_with_path_map(
        "agent",
        |state: &MessageState| {
            // If the last message has tool calls, go to tools
            if let Some(msg) = state.last_message() {
                if !msg.tool_calls().is_empty() {
                    return "tools".to_string();
                }
            }
            END.to_string()
        },
        HashMap::from([
            ("tools".to_string(), "tools".to_string()),
            (END.to_string(), END.to_string()),
        ]),
    )
    .add_edge("tools", "agent")  // tool results go back to agent
    .compile()?;
```

This is exactly the pattern that `create_react_agent()` implements automatically.

## `create_react_agent`

For convenience, Synapse provides a factory function that assembles the standard ReAct agent graph:

```rust
use synaptic_graph::create_react_agent;

let graph = create_react_agent(model, tools);
```

This creates a compiled graph with `"agent"` and `"tools"` nodes wired in a conditional loop, equivalent to the manual setup shown above.
