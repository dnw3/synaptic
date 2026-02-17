# Custom Tools

Every tool in Synapse implements the `Tool` trait from `synapse-core`. This page shows how to define your own tools.

## The Tool Trait

The `Tool` trait requires three methods:

```rust
use async_trait::async_trait;
use serde_json::Value;
use synapse_core::SynapseError;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name used to identify this tool in registries and tool calls.
    fn name(&self) -> &'static str;

    /// Human-readable description sent to the model so it understands what this tool does.
    fn description(&self) -> &'static str;

    /// Execute the tool with the given JSON arguments and return a JSON result.
    async fn call(&self, args: Value) -> Result<Value, SynapseError>;
}
```

## Implementing a Tool

Here is a complete example of a weather tool:

```rust
use async_trait::async_trait;
use serde_json::{json, Value};
use synapse_core::{Tool, SynapseError};

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &'static str {
        "get_weather"
    }

    fn description(&self) -> &'static str {
        "Get the current weather for a location"
    }

    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        let location = args["location"]
            .as_str()
            .unwrap_or("unknown");

        // In production, call a real weather API here
        Ok(json!({
            "location": location,
            "temperature": 22,
            "condition": "sunny"
        }))
    }
}
```

Key points:

- The `#[async_trait]` attribute is required because `Tool` is an async trait.
- `name()` returns a `&'static str` -- this is the identifier the model uses when making tool calls.
- `description()` tells the model what the tool does. Write clear, concise descriptions so the model knows when to use this tool.
- `call()` receives arguments as a `serde_json::Value` (typically a JSON object) and returns a `Value` result.

## Error Handling

Return `SynapseError::Tool(...)` for tool-specific errors:

```rust
use async_trait::async_trait;
use serde_json::{json, Value};
use synapse_core::{Tool, SynapseError};

struct DivisionTool;

#[async_trait]
impl Tool for DivisionTool {
    fn name(&self) -> &'static str {
        "divide"
    }

    fn description(&self) -> &'static str {
        "Divide two numbers"
    }

    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        let a = args["a"].as_f64()
            .ok_or_else(|| SynapseError::Tool("missing argument 'a'".to_string()))?;
        let b = args["b"].as_f64()
            .ok_or_else(|| SynapseError::Tool("missing argument 'b'".to_string()))?;

        if b == 0.0 {
            return Err(SynapseError::Tool("division by zero".to_string()));
        }

        Ok(json!({"result": a / b}))
    }
}
```

## Registering and Using

Once defined, wrap the tool in an `Arc` and register it:

```rust
use std::sync::Arc;
use synapse_tools::{ToolRegistry, SerialToolExecutor};
use serde_json::json;

let registry = ToolRegistry::new();
registry.register(Arc::new(WeatherTool))?;

let executor = SerialToolExecutor::new(registry);
let result = executor.execute("get_weather", json!({"location": "Tokyo"})).await?;
// result = {"location": "Tokyo", "temperature": 22, "condition": "sunny"}
```

See the [Tool Registry](registry.md) page for more on registration and execution.

## Tool Definitions for Models

To tell a chat model about available tools, create `ToolDefinition` values and attach them to a `ChatRequest`:

```rust
use serde_json::json;
use synapse_core::{ChatRequest, Message, ToolDefinition};

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
