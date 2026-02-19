# Tool Definition Extras

The `extras` field on `ToolDefinition` carries provider-specific parameters that fall outside the standard name/description/parameters schema, such as Anthropic's `cache_control` or any custom metadata your provider adapter needs.

## The `extras` Field

```rust,ignore
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    /// Provider-specific parameters (e.g., Anthropic's `cache_control`).
    pub extras: Option<HashMap<String, Value>>,
}
```

When `extras` is `None` (the default), no additional fields are serialized. Provider adapters inspect `extras` during request building and map recognized keys into the provider's wire format.

## Setting Extras on a Tool Definition

Build a `ToolDefinition` with extras by populating the field directly:

```rust,ignore
use std::collections::HashMap;
use serde_json::{json, Value};
use synaptic::core::ToolDefinition;

let mut extras = HashMap::new();
extras.insert("cache_control".to_string(), json!({"type": "ephemeral"}));

let tool_def = ToolDefinition {
    name: "search".to_string(),
    description: "Search the web".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" }
        },
        "required": ["query"]
    }),
    extras: Some(extras),
};
```

## Common Use Cases

**Anthropic prompt caching** -- Anthropic supports a `cache_control` field on tool definitions to enable prompt caching for tool schemas that rarely change:

```rust,ignore
let mut extras = HashMap::new();
extras.insert("cache_control".to_string(), json!({"type": "ephemeral"}));

let def = ToolDefinition {
    name: "lookup".to_string(),
    description: "Look up a record".to_string(),
    parameters: json!({"type": "object", "properties": {}}),
    extras: Some(extras),
};
```

**Custom metadata** -- You can attach arbitrary key-value pairs for your own adapter logic:

```rust,ignore
let mut extras = HashMap::new();
extras.insert("priority".to_string(), json!("high"));
extras.insert("timeout_ms".to_string(), json!(5000));

let def = ToolDefinition {
    name: "deploy".to_string(),
    description: "Deploy the service".to_string(),
    parameters: json!({"type": "object", "properties": {}}),
    extras: Some(extras),
};
```

## Extras with `#[tool]` Macro Tools

The `#[tool]` macro does not support `extras` directly -- extras are a property of the `ToolDefinition`, not the tool function itself. Define your tool with the macro, then add extras to the generated definition:

```rust,ignore
use std::collections::HashMap;
use serde_json::{json, Value};
use synaptic::macros::tool;
use synaptic::core::SynapticError;

/// Does something useful.
#[tool]
async fn my_tool(
    /// The input query
    query: String,
) -> Result<Value, SynapticError> {
    Ok(json!("done"))
}

// Get the tool definition and add extras
let tool = my_tool();
let mut def = tool.as_tool_definition();
def.extras = Some(HashMap::from([
    ("cache_control".to_string(), json!({"type": "ephemeral"})),
]));

// Use `def` when building the ChatRequest
```

This approach works with any tool -- whether defined via `#[tool]` or by implementing the `Tool` trait manually.
