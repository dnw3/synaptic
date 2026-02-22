# DeepSeek

[DeepSeek](https://deepseek.com/) offers powerful language and reasoning models at exceptionally low cost. DeepSeek models are often 90% or more cheaper than comparable proprietary models like GPT-4o, while matching or exceeding their performance on many benchmarks.

The DeepSeek API is fully compatible with the OpenAI API format. The `synaptic-deepseek` crate wraps `synaptic-openai` with the DeepSeek base URL preset and a type-safe `DeepSeekModel` enum.

## Setup

Add the `deepseek` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["deepseek"] }
```

Obtain an API key from [platform.deepseek.com](https://platform.deepseek.com/). Keys are prefixed with `sk-`.

## Configuration

Create a `DeepSeekConfig` with your API key and a `DeepSeekModel` variant:

```rust,ignore
use synaptic::deepseek::{DeepSeekChatModel, DeepSeekConfig, DeepSeekModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = DeepSeekConfig::new("sk-your-api-key", DeepSeekModel::DeepSeekChat);
let model = DeepSeekChatModel::new(config, Arc::new(HttpBackend::new()));
```

### Builder methods

`DeepSeekConfig` supports the standard fluent builder pattern:

```rust,ignore
let config = DeepSeekConfig::new("sk-key", DeepSeekModel::DeepSeekChat)
    .with_temperature(0.3)
    .with_max_tokens(4096)
    .with_top_p(0.9);
```

For unlisted models:

```rust,ignore
let config = DeepSeekConfig::new_custom("sk-key", "deepseek-chat");
```

## Available Models

| Enum Variant | API Model ID | Context | Best For |
|---|---|---|---|
| `DeepSeekChat` | `deepseek-chat` | 64 K | General-purpose, ultra-low cost |
| `DeepSeekReasoner` | `deepseek-reasoner` | 64 K | Chain-of-thought reasoning (R1) |
| `DeepSeekCoderV2` | `deepseek-coder-v2` | 128 K | Code generation and analysis |
| `Custom(String)` | _(any)_ | -- | Unlisted / preview models |

### Cost comparison

DeepSeek-V3 (`DeepSeekChat`) is priced at approximately /bin/zsh.27 per million output tokens, compared to  per million for GPT-4o. This makes DeepSeek an excellent choice for high-volume workloads and experimentation.

### DeepSeek-R1 reasoning model

The `DeepSeekReasoner` model (R1) uses chain-of-thought reasoning to solve complex problems. It shows its work in a `<think>` block before giving the final answer, which can be particularly useful for mathematics, coding challenges, and logical reasoning tasks.

## Usage

`DeepSeekChatModel` implements the `ChatModel` trait:

```rust,ignore
use synaptic::deepseek::{DeepSeekChatModel, DeepSeekConfig, DeepSeekModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = DeepSeekConfig::new("sk-key", DeepSeekModel::DeepSeekChat);
let model = DeepSeekChatModel::new(config, Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a concise technical assistant."),
    Message::human("Explain Rust's borrow checker in one sentence."),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content().unwrap_or_default());
```

## Streaming

Use `stream_chat()` to receive tokens incrementally:

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Write a Rust function that parses JSON."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.content);
}
println!();
```

## Tool Calling

DeepSeek-V3 supports OpenAI-compatible tool calling:

```rust,ignore
use synaptic::core::{ChatRequest, Message, ToolDefinition, ToolChoice};
use serde_json::json;

let tools = vec![ToolDefinition {
    name: "calculate".to_string(),
    description: "Evaluate a mathematical expression.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": { "expression": {"type": "string"} },
        "required": ["expression"]
    }),
}];

let request = ChatRequest::new(vec![Message::human("What is 42 * 1337?")])
    .with_tools(tools)
    .with_tool_choice(ToolChoice::Auto);

let response = model.chat(request).await?;
for tc in response.message.tool_calls() {
    println!("Tool: {}, Args: {}", tc.name, tc.arguments);
}
```

## Error Handling

The `SynapticError::RateLimit` variant is returned when the API responds with HTTP 429:

```rust,ignore
use synaptic::core::SynapticError;

match model.chat(request).await {
    Ok(response) => println!("{}", response.message.content().unwrap_or_default()),
    Err(SynapticError::RateLimit(msg)) => eprintln!("Rate limited: {}", msg),
    Err(e) => return Err(e.into()),
}
```

## Configuration Reference

### DeepSeekConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | `String` | required | DeepSeek API key (`sk-...`) |
| `model` | `String` | from enum | API model identifier |
| `max_tokens` | `Option<u32>` | `None` | Maximum tokens to generate |
| `temperature` | `Option<f64>` | `None` | Sampling temperature (0.0-2.0) |
| `top_p` | `Option<f64>` | `None` | Nucleus sampling threshold |
| `stop` | `Option<Vec<String>>` | `None` | Stop sequences |
| `seed` | `Option<u64>` | `None` | Seed for reproducible output |
