# tool_calling_basic

Demonstrates the Synaptic tool system: defining a custom `Tool`, registering it in a `ToolRegistry`, and executing it through `SerialToolExecutor`.

## What it does

1. Defines an `EchoTool` that implements the `Tool` trait
2. Registers the tool in a `ToolRegistry`
3. Executes the tool by name with a JSON payload via `SerialToolExecutor`
4. Prints the tool output

## Run

```bash
cargo run -p tool_calling_basic
```

## Expected output

```
{"echo":{"message":"hello from synaptic"}}
```

## Key concepts

- **`Tool` trait** — implement `name()`, `description()`, and `call(args) -> Value` to define a tool
- **`ToolRegistry`** — thread-safe registry (`Arc<RwLock<HashMap>>`) for tool lookup by name
- **`SerialToolExecutor`** — executes tools sequentially by name with JSON arguments
