# Tool Registry

`ToolRegistry` is a thread-safe collection of tools, and `SerialToolExecutor` dispatches tool calls through the registry by name. Both are provided by the `synaptic-tools` crate.

## ToolRegistry

`ToolRegistry` stores tools in an `Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>`. It is `Clone` and can be shared across threads.

### Creating and Registering Tools

```rust,ignore
use serde_json::{json, Value};
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use synaptic::tools::ToolRegistry;

/// Echo back the input.
#[tool]
async fn echo(
    #[args] args: Value,
) -> Result<Value, SynapticError> {
    Ok(json!({"echo": args}))
}

let registry = ToolRegistry::new();
registry.register(echo())?;  // echo() returns Arc<dyn Tool>
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
use synaptic::tools::SerialToolExecutor;
use serde_json::json;

let executor = SerialToolExecutor::new(registry);

let result = executor.execute("echo", json!({"message": "hello"})).await?;
assert_eq!(result, json!({"echo": {"message": "hello"}}));
```

The `execute()` method:

1. Looks up the tool by name in the registry.
2. Calls `tool.call(args)` with the provided arguments.
3. Returns the result or `SynapticError::ToolNotFound` if the tool does not exist.

### Handling Unknown Tools

If you call `execute()` with a name that is not registered, it returns `SynapticError::ToolNotFound`:

```rust
let err = executor.execute("nonexistent", json!({})).await.unwrap_err();
assert!(matches!(err, synaptic::core::SynapticError::ToolNotFound(name) if name == "nonexistent"));
```

## Complete Example

Here is a full example that registers multiple tools and executes them:

```rust,ignore
use serde_json::{json, Value};
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use synaptic::tools::{ToolRegistry, SerialToolExecutor};

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

/// Multiply two numbers.
#[tool]
async fn multiply(
    /// First number
    a: f64,
    /// Second number
    b: f64,
) -> Result<Value, SynapticError> {
    Ok(json!({"result": a * b}))
}

#[tokio::main]
async fn main() -> Result<(), SynapticError> {
    let registry = ToolRegistry::new();
    registry.register(add())?;
    registry.register(multiply())?;

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
use synaptic::core::{Message, ToolCall};
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
