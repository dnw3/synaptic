# Tool Choice

`ToolChoice` controls whether and how a chat model selects tools when responding. It is defined in `synaptic-core` and attached to a `ChatRequest` via the `with_tool_choice()` builder method.

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
use synaptic::core::{ChatRequest, Message, ToolChoice, ToolDefinition};

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
use synaptic::core::{ChatRequest, Message, ToolChoice};

let request = ChatRequest::new(vec![
    Message::human("Hello, how are you?"),
])
.with_tools(tool_defs)
.with_tool_choice(ToolChoice::Auto);
```

### Required

Force tool usage. Useful in agent loops where the next step must be a tool call, or when you know the user's request requires tool invocation:

```rust
use synaptic::core::{ChatRequest, Message, ToolChoice};

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
use synaptic::core::{ChatRequest, Message, ToolChoice};

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
use synaptic::core::{ChatRequest, Message, ToolChoice};

let request = ChatRequest::new(vec![
    Message::human("Check the weather in London."),
])
.with_tools(tool_defs)
.with_tool_choice(ToolChoice::Specific("get_weather".to_string()));
// The model MUST call the "get_weather" tool specifically
```

## Complete Example

Here is a full example that creates tools, forces a specific tool call, and processes the result:

```rust,ignore
use serde_json::{json, Value};
use synaptic::macros::tool;
use synaptic::core::{
    ChatModel, ChatRequest, Message, SynapticError, Tool,
    ToolChoice,
};
use synaptic::tools::{ToolRegistry, SerialToolExecutor};

/// Perform arithmetic calculations.
#[tool]
async fn calculator(
    /// The arithmetic expression to evaluate
    expression: String,
) -> Result<Value, SynapticError> {
    // Simplified: in production, parse and evaluate the expression
    Ok(json!({"result": expression}))
}

// Register tools
let registry = ToolRegistry::new();
let calc_tool = calculator();  // Arc<dyn Tool>
registry.register(calc_tool.clone())?;

// Build the tool definition from the tool itself
let calc_def = calc_tool.as_tool_definition();

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

All Synaptic provider adapters (`OpenAiChatModel`, `AnthropicChatModel`, `GeminiChatModel`, `OllamaChatModel`) support `ToolChoice`. The adapter translates the Synaptic `ToolChoice` enum into the provider-specific format automatically.

See also: [Bind Tools](../chat-models/bind-tools.md) for attaching tools to a model permanently, and the [ReAct Agent tutorial](../../tutorials/react-agent.md) for a complete agent loop.
