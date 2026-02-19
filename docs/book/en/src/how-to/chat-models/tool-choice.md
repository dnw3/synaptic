# Control Tool Choice

This guide shows how to control whether and which tools the model uses when responding to a request.

## Overview

When you attach tools to a `ChatRequest`, the model decides by default whether to call any of them. The `ToolChoice` enum lets you override this behavior, forcing the model to use tools, avoid them, or target a specific one.

## The `ToolChoice` enum

```rust
use synaptic::core::ToolChoice;

// Auto -- the model decides whether to use tools (this is the default)
ToolChoice::Auto

// Required -- the model must call at least one tool
ToolChoice::Required

// None -- the model must not call any tools, even if tools are provided
ToolChoice::None

// Specific -- the model must call this exact tool
ToolChoice::Specific("get_weather".to_string())
```

## Setting tool choice on a request

Use `ChatRequest::with_tool_choice()`:

```rust
use synaptic::core::{ChatRequest, Message, ToolChoice, ToolDefinition};
use serde_json::json;

let tools = vec![
    ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get weather for a city".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" }
            },
            "required": ["city"]
        }),
    },
    ToolDefinition {
        name: "search".to_string(),
        description: "Search the web".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        }),
    },
];

let messages = vec![Message::human("What's the weather in London?")];
```

### Auto (default)

The model chooses freely whether to call tools:

```rust
let request = ChatRequest::new(messages.clone())
    .with_tools(tools.clone())
    .with_tool_choice(ToolChoice::Auto);
```

This is equivalent to not calling `with_tool_choice()` at all.

### Required

Force the model to call at least one tool. Useful when you know the user's intent maps to a tool call:

```rust
let request = ChatRequest::new(messages.clone())
    .with_tools(tools.clone())
    .with_tool_choice(ToolChoice::Required);
```

### None

Prevent the model from calling tools, even though tools are provided. This is helpful when you want to temporarily disable tool usage without removing the definitions:

```rust
let request = ChatRequest::new(messages.clone())
    .with_tools(tools.clone())
    .with_tool_choice(ToolChoice::None);
```

### Specific

Force the model to call one specific tool by name. The model will always call this tool, regardless of the conversation context:

```rust
let request = ChatRequest::new(messages.clone())
    .with_tools(tools.clone())
    .with_tool_choice(ToolChoice::Specific("get_weather".to_string()));
```

## Practical patterns

### Routing with specific tool choice

When building a multi-step agent, you can force a classification step by requiring a specific "router" tool:

```rust
let router_tool = ToolDefinition {
    name: "route".to_string(),
    description: "Classify the user's intent".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "intent": {
                "type": "string",
                "enum": ["weather", "search", "calculator"]
            }
        },
        "required": ["intent"]
    }),
};

let request = ChatRequest::new(vec![Message::human("What is 2 + 2?")])
    .with_tools(vec![router_tool])
    .with_tool_choice(ToolChoice::Specific("route".to_string()));
```

### Two-phase generation

First call with `Required` to extract structured data, then call with `None` to generate a natural language response:

```rust
// Phase 1: extract data
let extract_request = ChatRequest::new(messages.clone())
    .with_tools(tools.clone())
    .with_tool_choice(ToolChoice::Required);

// Phase 2: generate response (no tools)
let respond_request = ChatRequest::new(full_conversation)
    .with_tools(tools.clone())
    .with_tool_choice(ToolChoice::None);
```
