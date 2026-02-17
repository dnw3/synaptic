# Message Types

This guide covers all message variants in Synapse, how to create them, and how to inspect their contents.

## The `Message` enum

`Message` is a tagged enum (`#[serde(tag = "role")]`) with six variants:

| Variant | Factory method | Role string | Purpose |
|---------|---------------|-------------|---------|
| `System` | `Message::system()` | `"system"` | System instructions for the model |
| `Human` | `Message::human()` | `"human"` | User input |
| `AI` | `Message::ai()` | `"assistant"` | Model response (text only) |
| `AI` (with tools) | `Message::ai_with_tool_calls()` | `"assistant"` | Model response with tool calls |
| `Tool` | `Message::tool()` | `"tool"` | Tool execution result |
| `Chat` | `Message::chat()` | custom | Custom role message |
| `Remove` | `Message::remove()` | `"remove"` | Signals removal of a message by ID |

## Creating messages

Always use factory methods instead of constructing enum variants directly:

```rust
use synaptic_core::{Message, ToolCall};
use serde_json::json;

// System message -- sets the model's behavior
let system = Message::system("You are a helpful assistant.");

// Human message -- user input
let human = Message::human("Hello, how are you?");

// AI message -- plain text response
let ai = Message::ai("I'm doing well, thanks for asking!");

// AI message with tool calls
let ai_tools = Message::ai_with_tool_calls(
    "Let me look that up for you.",
    vec![
        ToolCall {
            id: "call_1".to_string(),
            name: "search".to_string(),
            arguments: json!({"query": "Rust programming"}),
        },
    ],
);

// Tool message -- result of a tool execution
// Second argument is the tool_call_id, which must match the ToolCall's id
let tool = Message::tool("Found 42 results for 'Rust programming'", "call_1");

// Chat message -- custom role
let chat = Message::chat("moderator", "This conversation is on topic.");

// Remove message -- used in message history management
let remove = Message::remove("msg-id-to-remove");
```

## Accessor methods

All message variants share a common set of accessor methods:

```rust
use synaptic_core::Message;

let msg = Message::human("Hello!");

// Get the role as a string
assert_eq!(msg.role(), "human");

// Get the text content
assert_eq!(msg.content(), "Hello!");

// Type-checking predicates
assert!(msg.is_human());
assert!(!msg.is_ai());
assert!(!msg.is_system());
assert!(!msg.is_tool());
assert!(!msg.is_chat());
assert!(!msg.is_remove());

// Tool-related accessors (empty/None for non-AI/non-Tool messages)
assert!(msg.tool_calls().is_empty());
assert!(msg.tool_call_id().is_none());

// Optional fields
assert!(msg.id().is_none());
assert!(msg.name().is_none());
```

### Tool call accessors

```rust
use synaptic_core::{Message, ToolCall};
use serde_json::json;

let ai = Message::ai_with_tool_calls("", vec![
    ToolCall {
        id: "call_1".into(),
        name: "search".into(),
        arguments: json!({"q": "rust"}),
    },
]);

// Get all tool calls (only meaningful for AI messages)
let calls = ai.tool_calls();
assert_eq!(calls.len(), 1);
assert_eq!(calls[0].name, "search");

let tool_msg = Message::tool("result", "call_1");

// Get the tool_call_id (only meaningful for Tool messages)
assert_eq!(tool_msg.tool_call_id(), Some("call_1"));
```

## Builder methods

Messages support a builder pattern for setting optional fields:

```rust
use synaptic_core::Message;
use serde_json::json;

let msg = Message::human("Hello!")
    .with_id("msg-001")
    .with_name("Alice")
    .with_additional_kwarg("source", json!("web"))
    .with_response_metadata_entry("model", json!("gpt-4o"));

assert_eq!(msg.id(), Some("msg-001"));
assert_eq!(msg.name(), Some("Alice"));
```

Available builder methods:

| Method | Description |
|--------|-------------|
| `.with_id(id)` | Set the message ID |
| `.with_name(name)` | Set the sender name |
| `.with_additional_kwarg(key, value)` | Add an arbitrary key-value pair |
| `.with_response_metadata_entry(key, value)` | Add response metadata |
| `.with_content_blocks(blocks)` | Set multimodal content blocks |
| `.with_usage_metadata(usage)` | Set token usage (AI messages only) |

## Serialization

Messages serialize to JSON with a `"role"` tag:

```rust
use synaptic_core::Message;

let msg = Message::human("Hello!");
let json = serde_json::to_string_pretty(&msg).unwrap();
// {
//   "role": "human",
//   "content": "Hello!"
// }
```

Note that the AI variant serializes with `"role": "assistant"` (not `"ai"`), matching the convention used by most LLM providers.
