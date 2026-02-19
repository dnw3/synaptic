# Custom Tools

Every tool in Synaptic implements the `Tool` trait from `synaptic-core`. The recommended way to define tools is with the `#[tool]` attribute macro, which generates all the boilerplate for you.

## Defining a Tool with `#[tool]`

The `#[tool]` macro converts an async function into a full `Tool` implementation. Doc comments on the function become the tool description, and doc comments on parameters become JSON Schema descriptions:

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use serde_json::{json, Value};

/// Get the current weather for a location.
#[tool]
async fn get_weather(
    /// The city name
    location: String,
) -> Result<Value, SynapticError> {
    // In production, call a real weather API here
    Ok(json!({
        "location": location,
        "temperature": 22,
        "condition": "sunny"
    }))
}

// `get_weather()` returns Arc<dyn Tool>
let tool = get_weather();
assert_eq!(tool.name(), "get_weather");
```

Key points:

- The function name becomes the tool name (override with `#[tool(name = "custom_name")]`).
- The doc comment on the function becomes the tool description.
- Each parameter becomes a JSON Schema property; doc comments on parameters become `"description"` fields in the schema.
- `String`, `i64`, `f64`, `bool`, `Vec<T>`, and `Option<T>` types are mapped to JSON Schema types automatically.
- The factory function (`get_weather()`) returns `Arc<dyn Tool>`.

## Error Handling

Return `SynapticError::Tool(...)` for tool-specific errors. The macro handles parameter validation automatically, but you can add your own domain-specific checks:

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use serde_json::{json, Value};

/// Divide two numbers.
#[tool]
async fn divide(
    /// The numerator
    a: f64,
    /// The denominator
    b: f64,
) -> Result<Value, SynapticError> {
    if b == 0.0 {
        return Err(SynapticError::Tool("division by zero".to_string()));
    }

    Ok(json!({"result": a / b}))
}
```

Note that the macro auto-generates validation for missing or invalid parameters (returning `SynapticError::Tool` errors), so you no longer need manual `args["a"].as_f64().ok_or_else(...)` checks.

## Registering and Using

The `#[tool]` macro factory returns `Arc<dyn Tool>`, which you register directly:

```rust,ignore
use synaptic::tools::{ToolRegistry, SerialToolExecutor};
use serde_json::json;

let registry = ToolRegistry::new();
registry.register(get_weather())?;

let executor = SerialToolExecutor::new(registry);
let result = executor.execute("get_weather", json!({"location": "Tokyo"})).await?;
// result = {"location": "Tokyo", "temperature": 22, "condition": "sunny"}
```

See the [Tool Registry](registry.md) page for more on registration and execution.

## Full ReAct Agent Loop

Here is a complete offline example that defines tools with `#[tool]`, then wires them into a ReAct agent with `ScriptedChatModel`:

```rust,ignore
use std::sync::Arc;
use serde_json::{json, Value};
use synaptic::macros::tool;
use synaptic::core::{ChatModel, ChatResponse, Message, Tool, ToolCall, SynapticError};
use synaptic::models::ScriptedChatModel;
use synaptic::graph::{create_react_agent, MessageState};

// 1. Define tools with the macro
/// Add two numbers.
#[tool]
async fn add(
    /// First number
    a: f64,
    /// Second number
    b: f64,
) -> Result<Value, SynapticError> {
    Ok(json!({"result": a + b}))
}

// 2. Script the model to call the tool and then respond
let model: Arc<dyn ChatModel> = Arc::new(ScriptedChatModel::new(vec![
    ChatResponse {
        message: Message::ai_with_tool_calls(
            "",
            vec![ToolCall {
                id: "call_1".into(),
                name: "add".into(),
                arguments: r#"{"a": 3, "b": 4}"#.into(),
            }],
        ),
        usage: None,
    },
    ChatResponse {
        message: Message::ai("The sum is 7."),
        usage: None,
    },
]));

// 3. Build the agent -- add() returns Arc<dyn Tool>
let tools: Vec<Arc<dyn Tool>> = vec![add()];
let agent = create_react_agent(model, tools)?;

// 4. Run it
let state = MessageState::with_messages(vec![
    Message::human("What is 3 + 4?"),
]);
let result = agent.invoke(state).await?.into_state();
assert_eq!(result.messages.last().unwrap().content(), "The sum is 7.");
```

## Tool Definitions for Models

To tell a chat model about available tools, create `ToolDefinition` values and attach them to a `ChatRequest`:

```rust
use serde_json::json;
use synaptic::core::{ChatRequest, Message, ToolDefinition};

let tool_def = ToolDefinition {
    name: "get_weather".to_string(),
    description: "Get the current weather for a location".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "The city name"
            }
        },
        "required": ["location"]
    }),
};

let request = ChatRequest::new(vec![
    Message::human("What is the weather in Tokyo?"),
])
.with_tools(vec![tool_def]);
```

The `parameters` field follows the JSON Schema format that LLM providers expect.

## Optional and Default Parameters

```rust,ignore
#[tool]
async fn search(
    /// The search query
    query: String,
    /// Maximum results (default 10)
    #[default = 10]
    max_results: i64,
    /// Language filter
    language: Option<String>,
) -> Result<String, SynapticError> {
    let lang = language.unwrap_or_else(|| "en".into());
    Ok(format!("Searching '{}' (max {}, lang {})", query, max_results, lang))
}
```

## Stateful Tools with `#[field]`

Tools that need to hold state (database connections, API clients, etc.) can use
`#[field]` to create struct fields that are hidden from the LLM schema:

```rust,ignore
use std::sync::Arc;

#[tool]
async fn db_query(
    #[field] pool: Arc<DbPool>,
    /// SQL query to execute
    query: String,
) -> Result<Value, SynapticError> {
    let result = pool.execute(&query).await?;
    Ok(serde_json::to_value(result).unwrap())
}

// Factory requires the field parameter
let tool = db_query(pool.clone());
```

For the full macro reference including `#[inject]`, `#[default]`, and middleware
macros, see the [Procedural Macros](../macros.md) page.

## Manual Implementation

For advanced cases that the macro cannot handle (custom `parameters()` overrides, conditional logic in `name()` or `description()`, or implementing both `Tool` and other traits on the same struct), you can implement the `Tool` trait directly:

```rust
use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic::core::{Tool, SynapticError};

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &'static str {
        "get_weather"
    }

    fn description(&self) -> &'static str {
        "Get the current weather for a location"
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let location = args["location"]
            .as_str()
            .unwrap_or("unknown");

        Ok(json!({
            "location": location,
            "temperature": 22,
            "condition": "sunny"
        }))
    }
}
```

The trait requires three methods:

- `name()` -- a `&'static str` identifier the model uses when making tool calls.
- `description()` -- tells the model what the tool does.
- `call()` -- receives arguments as a `serde_json::Value` and returns a `Value` result.

Wrap manual implementations in `Arc::new(WeatherTool)` when registering them.
