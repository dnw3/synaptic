# Merge Message Runs

This guide shows how to use `merge_message_runs` to combine consecutive messages of the same role into a single message.

## Overview

Some LLM providers require alternating message roles (human, assistant, human, assistant). If your message history has consecutive messages from the same role, you can merge them into one message before sending the request.

## Basic usage

```rust
use synaptic_core::{merge_message_runs, Message};

let messages = vec![
    Message::human("Hello"),
    Message::human("How are you?"),       // Same role as previous
    Message::ai("I'm fine!"),
    Message::ai("Thanks for asking!"),    // Same role as previous
];

let merged = merge_message_runs(messages);

assert_eq!(merged.len(), 2);
assert_eq!(merged[0].content(), "Hello\nHow are you?");
assert_eq!(merged[1].content(), "I'm fine!\nThanks for asking!");
```

## How merging works

When two consecutive messages share the same role:

1. Their `content` strings are joined with a newline (`\n`).
2. For AI messages, `tool_calls` and `invalid_tool_calls` from subsequent messages are appended to the first message's lists.
3. The resulting message retains the `id`, `name`, and other metadata of the first message in the run.

## Merging AI messages with tool calls

Tool calls from consecutive AI messages are combined:

```rust
use synaptic_core::{merge_message_runs, Message, ToolCall};
use serde_json::json;

let messages = vec![
    Message::ai_with_tool_calls("Looking up weather...", vec![
        ToolCall {
            id: "call_1".into(),
            name: "get_weather".into(),
            arguments: json!({"city": "Tokyo"}),
        },
    ]),
    Message::ai_with_tool_calls("Also checking news...", vec![
        ToolCall {
            id: "call_2".into(),
            name: "search_news".into(),
            arguments: json!({"query": "Tokyo"}),
        },
    ]),
];

let merged = merge_message_runs(messages);

assert_eq!(merged.len(), 1);
assert_eq!(merged[0].content(), "Looking up weather...\nAlso checking news...");
assert_eq!(merged[0].tool_calls().len(), 2);
```

## Preserving different roles

Messages with different roles are never merged, even if they appear to be related:

```rust
use synaptic_core::{merge_message_runs, Message};

let messages = vec![
    Message::system("Be helpful."),
    Message::human("Hi"),
    Message::ai("Hello!"),
    Message::human("Bye"),
];

let merged = merge_message_runs(messages);
assert_eq!(merged.len(), 4);  // No change -- all roles are different
```

## Practical use case: preparing messages for providers

Some providers reject requests with consecutive same-role messages. Use `merge_message_runs` to clean up before sending:

```rust
use synaptic_core::{merge_message_runs, ChatRequest, Message};

let conversation = vec![
    Message::system("You are a translator."),
    Message::human("Translate to French:"),
    Message::human("Hello, how are you?"),    // User sent two messages in a row
    Message::ai("Bonjour, comment allez-vous ?"),
];

let cleaned = merge_message_runs(conversation);
let request = ChatRequest::new(cleaned);
// Now safe to send: roles alternate correctly
```

## Empty input

`merge_message_runs` returns an empty vector when given an empty input:

```rust
use synaptic_core::merge_message_runs;

let result = merge_message_runs(vec![]);
assert!(result.is_empty());
```
