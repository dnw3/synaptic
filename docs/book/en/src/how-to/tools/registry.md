# Tool Registry

`ToolRegistry` is a thread-safe collection of tools, and `SerialToolExecutor` dispatches tool calls through the registry by name. Both are provided by the `synaptic-tools` crate.

## ToolRegistry

`ToolRegistry` stores tools in an `Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>`. It is `Clone` and can be shared across threads.

### Creating and Registering Tools

```rust
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{Tool, SynapseError};
use synaptic_tools::ToolRegistry;

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str { "echo" }
    fn description(&self) -> &'static str { "Echo back the input" }
    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        Ok(json!({"echo": args}))
    }
}

let registry = ToolRegistry::new();
registry.register(Arc::new(EchoTool))?;
```

If you register two tools with the same name, the second registration replaces the first.

### Looking Up Tools

Use `get()` to retrieve a tool by name:

```rust
let tool = registry.get("echo");
assert!(tool.is_some());

let missing = registry.get("nonexistent");
assert!(missing.is_none());
```

`get()` returns `Option<Arc<dyn Tool>>`, so the tool can be called directly if needed.

## SerialToolExecutor

`SerialToolExecutor` wraps a `ToolRegistry` and provides a convenience method that looks up a tool by name and calls it in one step.

### Creating and Using

```rust
use synaptic_tools::SerialToolExecutor;
use serde_json::json;

let executor = SerialToolExecutor::new(registry);

let result = executor.execute("echo", json!({"message": "hello"})).await?;
assert_eq!(result, json!({"echo": {"message": "hello"}}));
```

The `execute()` method:

1. Looks up the tool by name in the registry.
2. Calls `tool.call(args)` with the provided arguments.
3. Returns the result or `SynapseError::ToolNotFound` if the tool does not exist.

### Handling Unknown Tools

If you call `execute()` with a name that is not registered, it returns `SynapseError::ToolNotFound`:

```rust
let err = executor.execute("nonexistent", json!({})).await.unwrap_err();
assert!(matches!(err, synaptic_core::SynapseError::ToolNotFound(name) if name == "nonexistent"));
```

## Complete Example

Here is a full example that registers multiple tools and executes them:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{Tool, SynapseError};
use synaptic_tools::{ToolRegistry, SerialToolExecutor};

struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &'static str { "add" }
    fn description(&self) -> &'static str { "Add two numbers" }
    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        Ok(json!({"result": a + b}))
    }
}

struct MultiplyTool;

#[async_trait]
impl Tool for MultiplyTool {
    fn name(&self) -> &'static str { "multiply" }
    fn description(&self) -> &'static str { "Multiply two numbers" }
    async fn call(&self, args: Value) -> Result<Value, SynapseError> {
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        Ok(json!({"result": a * b}))
    }
}

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    let registry = ToolRegistry::new();
    registry.register(Arc::new(AddTool))?;
    registry.register(Arc::new(MultiplyTool))?;

    let executor = SerialToolExecutor::new(registry);

    let sum = executor.execute("add", json!({"a": 3, "b": 4})).await?;
    assert_eq!(sum, json!({"result": 7.0}));

    let product = executor.execute("multiply", json!({"a": 3, "b": 4})).await?;
    assert_eq!(product, json!({"result": 12.0}));

    Ok(())
}
```

## Integration with Chat Models

In a typical agent workflow, the model's response contains `ToolCall` entries. You dispatch them through the executor and send the results back:

```rust
use synaptic_core::{Message, ToolCall};
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

See the [ReAct Agent tutorial](../../tutorials/react-agent.md) for a complete agent loop example.
