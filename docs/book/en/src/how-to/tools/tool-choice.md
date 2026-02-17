# Tool Choice

`ToolChoice` controls whether and how a chat model selects tools when responding. It is defined in `synapse-core` and attached to a `ChatRequest` via the `with_tool_choice()` builder method.

## ToolChoice Variants

| Variant | Behavior |
|---------|----------|
| `ToolChoice::Auto` | The model decides whether to call a tool or respond with text (default when tools are provided) |
| `ToolChoice::Required` | The model must call at least one tool -- it cannot respond with plain text |
| `ToolChoice::None` | The model must not call any tools, even if tools are provided in the request |
| `ToolChoice::Specific(name)` | The model must call the specific named tool |

## Basic Usage

Attach `ToolChoice` to a `ChatRequest` alongside tool definitions:

```rust
use serde_json::json;
use synapse_core::{ChatRequest, Message, ToolChoice, ToolDefinition};

let weather_tool = ToolDefinition {
    name: "get_weather".to_string(),
    description: "Get the current weather for a location".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "location": { "type": "string" }
        },
        "required": ["location"]
    }),
};

// Force the model to use tools
let request = ChatRequest::new(vec![
    Message::human("What is the weather in Tokyo?"),
])
.with_tools(vec![weather_tool])
.with_tool_choice(ToolChoice::Required);
```

## When to Use Each Variant

### Auto (Default)

Let the model decide. This is the best choice for general-purpose agents that should respond with text when no tool is needed:

```rust
use synapse_core::{ChatRequest, Message, ToolChoice};

let request = ChatRequest::new(vec![
    Message::human("Hello, how are you?"),
])
.with_tools(tool_defs)
.with_tool_choice(ToolChoice::Auto);
```

### Required

Force tool usage. Useful in agent loops where the next step must be a tool call, or when you know the user's request requires tool invocation:

```rust
use synapse_core::{ChatRequest, Message, ToolChoice};

let request = ChatRequest::new(vec![
    Message::human("Look up the weather in Paris and Tokyo."),
])
.with_tools(tool_defs)
.with_tool_choice(ToolChoice::Required);
// The model MUST respond with one or more tool calls
```

### None

Suppress tool calls. Useful when you want to temporarily disable tools without removing them from the request, or during a final summarization step:

```rust
use synapse_core::{ChatRequest, Message, ToolChoice};

let request = ChatRequest::new(vec![
    Message::system("Summarize the tool results for the user."),
    Message::human("What is the weather?"),
    // ... tool result messages ...
])
.with_tools(tool_defs)
.with_tool_choice(ToolChoice::None);
// The model MUST respond with text, not tool calls
```

### Specific

Force a particular tool. Useful when you know exactly which tool should be called:

```rust
use synapse_core::{ChatRequest, Message, ToolChoice};

let request = ChatRequest::new(vec![
    Message::human("Check the weather in London."),
])
.with_tools(tool_defs)
.with_tool_choice(ToolChoice::Specific("get_weather".to_string()));
// The model MUST call the "get_weather" tool specifically
```

## Complete Example

Here is a full example that creates tools, forces a specific tool call, and processes the result:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use synapse_core::{
    ChatModel, ChatRequest, Message, SynapseError, Tool,
    ToolChoice, ToolDefinition,
};
use synapse_tools::{ToolRegistry, SerialToolExecutor};

// Define the tool
struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &'static str { "calculator" }
    fn description(&self) -> &'static str { "Perform arithmetic calculations" }
    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        let expr = args["expression"].as_str().unwrap_or("");
        // Simplified: in production, parse and evaluate the expression
        Ok(json!({"result": expr}))
    }
}

// Register tools
let registry = ToolRegistry::new();
registry.register(Arc::new(CalculatorTool))?;

// Build the tool definition for the model
let calc_def = ToolDefinition {
    name: "calculator".to_string(),
    description: "Perform arithmetic calculations".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "expression": {
                "type": "string",
                "description": "The arithmetic expression to evaluate"
            }
        },
        "required": ["expression"]
    }),
};

// Build a request that forces the calculator tool
let request = ChatRequest::new(vec![
    Message::human("What is 42 * 17?"),
])
.with_tools(vec![calc_def])
.with_tool_choice(ToolChoice::Specific("calculator".to_string()));

// Send to the model, then execute the returned tool calls
let response = model.chat(request).await?;
for tc in response.message.tool_calls() {
    let executor = SerialToolExecutor::new(registry.clone());
    let result = executor.execute(&tc.name, tc.arguments.clone()).await?;
    println!("Tool {} returned: {}", tc.name, result);
}
```

## Provider Support

All Synapse provider adapters (`OpenAiChatModel`, `AnthropicChatModel`, `GeminiChatModel`, `OllamaChatModel`) support `ToolChoice`. The adapter translates the Synapse `ToolChoice` enum into the provider-specific format automatically.

See also: [Bind Tools](../chat-models/bind-tools.md) for attaching tools to a model permanently, and the [ReAct Agent tutorial](../../tutorials/react-agent.md) for a complete agent loop.
