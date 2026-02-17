# Streaming Responses

This guide shows how to consume LLM responses as a stream of tokens, rather than waiting for the entire response to complete.

## Overview

Every `ChatModel` in Synapse provides two methods:

- `chat()` -- returns a complete `ChatResponse` once the model finishes generating.
- `stream_chat()` -- returns a `ChatStream`, which yields `AIMessageChunk` items as the model produces them.

Streaming is useful for displaying partial results to users in real time.

## Basic streaming

Use `stream_chat()` and iterate over chunks with `StreamExt::next()`:

```rust
use futures::StreamExt;
use synapse_core::{ChatModel, ChatRequest, Message, AIMessageChunk};

async fn stream_example(model: &dyn ChatModel) -> Result<(), Box<dyn std::error::Error>> {
    let request = ChatRequest::new(vec![
        Message::human("Tell me a story about a brave robot"),
    ]);

    let mut stream = model.stream_chat(request);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        print!("{}", chunk.content);  // Print each token as it arrives
    }
    println!();  // Final newline

    Ok(())
}
```

The `ChatStream` type is defined as:

```rust
type ChatStream<'a> = Pin<Box<dyn Stream<Item = Result<AIMessageChunk, SynapseError>> + Send + 'a>>;
```

## Accumulating chunks into a message

`AIMessageChunk` supports the `+` and `+=` operators for merging chunks together. After streaming completes, convert the accumulated result into a full `Message`:

```rust
use futures::StreamExt;
use synapse_core::{ChatModel, ChatRequest, Message, AIMessageChunk};

async fn accumulate_stream(model: &dyn ChatModel) -> Result<Message, Box<dyn std::error::Error>> {
    let request = ChatRequest::new(vec![
        Message::human("Summarize Rust's ownership model"),
    ]);

    let mut stream = model.stream_chat(request);
    let mut full = AIMessageChunk::default();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full += chunk;  // Merge content, tool_calls, usage, etc.
    }

    let final_message = full.into_message();
    println!("Complete response: {}", final_message.content());

    Ok(final_message)
}
```

When merging chunks:
- `content` strings are concatenated.
- `tool_calls` are appended to the accumulated list.
- `usage` token counts are summed.
- The first non-`None` `id` is preserved.

## Using the `+` operator

You can also combine two chunks with `+` without mutation:

```rust
let combined = chunk_a + chunk_b;
```

This produces a new `AIMessageChunk` with the merged fields from both.

## Streaming with tool calls

When the model streams a response that includes tool calls, tool call data arrives across multiple chunks. After accumulation, the full tool call information is available on the resulting message:

```rust
use futures::StreamExt;
use synapse_core::{ChatModel, ChatRequest, Message, AIMessageChunk, ToolDefinition};
use serde_json::json;

async fn stream_with_tools(model: &dyn ChatModel) -> Result<(), Box<dyn std::error::Error>> {
    let tool = ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        parameters: json!({"type": "object", "properties": {"city": {"type": "string"}}}),
    };

    let request = ChatRequest::new(vec![
        Message::human("What's the weather in Paris?"),
    ]).with_tools(vec![tool]);

    let mut stream = model.stream_chat(request);
    let mut full = AIMessageChunk::default();

    while let Some(chunk) = stream.next().await {
        full += chunk?;
    }

    let message = full.into_message();
    for tc in message.tool_calls() {
        println!("Call tool '{}' with: {}", tc.name, tc.arguments);
    }

    Ok(())
}
```

## Default streaming behavior

If a provider adapter does not implement native streaming, the default `stream_chat()` implementation wraps the `chat()` result as a single-chunk stream. This means you can always use `stream_chat()` regardless of provider -- you just may not get incremental token delivery from providers that do not support it natively.
