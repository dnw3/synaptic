# Bind Tools to a Model

This guide shows how to include tool (function) definitions in a chat request so the model can decide to call them.

## Defining tools

A `ToolDefinition` describes a tool the model can invoke. It has a name, description, and a JSON Schema for its parameters:

```rust
use synaptic_core::ToolDefinition;
use serde_json::json;

let weather_tool = ToolDefinition {
    name: "get_weather".to_string(),
    description: "Get the current weather for a location".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "City name, e.g. 'Tokyo'"
            }
        },
        "required": ["location"]
    }),
};
```

## Sending tools with a request

Use `ChatRequest::with_tools()` to attach tool definitions to a single request:

```rust
use synaptic_core::{ChatModel, ChatRequest, Message, ToolDefinition};
use serde_json::json;

async fn call_with_tools(model: &dyn ChatModel) -> Result<(), Box<dyn std::error::Error>> {
    let tool_def = ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get the current weather for a location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    };

    let request = ChatRequest::new(vec![
        Message::human("What's the weather in Tokyo?"),
    ]).with_tools(vec![tool_def]);

    let response = model.chat(request).await?;

    // Check if the model decided to call any tools
    for tc in response.message.tool_calls() {
        println!("Tool: {}, Args: {}", tc.name, tc.arguments);
    }

    Ok(())
}
```

## Processing tool calls

When the model returns tool calls, each `ToolCall` contains:

- `id` -- a unique identifier for this call (used to match the tool result back)
- `name` -- the name of the tool to invoke
- `arguments` -- a `serde_json::Value` with the arguments

After executing the tool, send the result back as a `Tool` message:

```rust
use synaptic_core::{ChatRequest, Message, ToolCall};
use serde_json::json;

// Suppose the model returned a tool call
let tool_call = ToolCall {
    id: "call_123".to_string(),
    name: "get_weather".to_string(),
    arguments: json!({"location": "Tokyo"}),
};

// Execute your tool logic...
let result = "Sunny, 22C";

// Send the result back in a follow-up request
let messages = vec![
    Message::human("What's the weather in Tokyo?"),
    Message::ai_with_tool_calls("", vec![tool_call]),
    Message::tool(result, "call_123"),  // tool_call_id must match
];

let follow_up = ChatRequest::new(messages);
// let final_response = model.chat(follow_up).await?;
```

## Permanently binding tools with `BoundToolsChatModel`

If you want every request through a model to automatically include certain tool definitions, use `BoundToolsChatModel`:

```rust
use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, ToolDefinition};
use synaptic_models::BoundToolsChatModel;
use serde_json::json;

let tools = vec![
    ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get weather for a city".to_string(),
        parameters: json!({"type": "object", "properties": {"city": {"type": "string"}}}),
    },
    ToolDefinition {
        name: "search".to_string(),
        description: "Search the web".to_string(),
        parameters: json!({"type": "object", "properties": {"query": {"type": "string"}}}),
    },
];

let base_model: Arc<dyn ChatModel> = Arc::new(base_model);
let bound = BoundToolsChatModel::new(base_model, tools);

// Now every call to bound.chat() will include both tools automatically
let request = ChatRequest::new(vec![Message::human("Look up Rust news")]);
// let response = bound.chat(request).await?;
```

## Multiple tools

You can provide any number of tools. The model will choose which (if any) to call based on the conversation context:

```rust
let request = ChatRequest::new(vec![
    Message::human("Search for Rust news and tell me the weather in Berlin"),
]).with_tools(vec![search_tool, weather_tool, calculator_tool]);
```

See also: [Control Tool Choice](tool-choice.md) for fine-grained control over which tools the model uses.
