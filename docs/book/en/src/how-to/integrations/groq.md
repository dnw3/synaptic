# Groq

[Groq](https://groq.com/) delivers ultra-fast LLM inference using their proprietary LPU (Language Processing Unit) hardware. Response speeds regularly exceed 500 tokens per second, making Groq ideal for real-time applications, interactive agents, and latency-sensitive pipelines.

The Groq API is fully compatible with the OpenAI API format. The `synaptic-groq` crate wraps `synaptic-openai` with the Groq base URL preset and a type-safe model name enum.

## Setup

Add the `groq` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["groq"] }
```

Sign up at [console.groq.com](https://console.groq.com/) to obtain an API key. Keys are prefixed with `gsk-`.

## Configuration

Create a `GroqConfig` with your API key and a `GroqModel` variant:

```rust,ignore
use synaptic::groq::{GroqChatModel, GroqConfig, GroqModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = GroqConfig::new("gsk-your-api-key", GroqModel::Llama3_3_70bVersatile);
let model = GroqChatModel::new(config, Arc::new(HttpBackend::new()));
```

### Builder methods

`GroqConfig` exposes a fluent builder for optional parameters:

```rust,ignore
let config = GroqConfig::new("gsk-key", GroqModel::Llama3_3_70bVersatile)
    .with_temperature(0.7)
    .with_max_tokens(2048)
    .with_top_p(0.9)
    .with_seed(42)
    .with_stop(vec\!["<|end|>".to_string()]);
```

To use a model not yet listed in `GroqModel`, use the custom variant:

```rust,ignore
let config = GroqConfig::new_custom("gsk-key", "llama-3.1-405b");
```

## Available Models

| Enum Variant | API Model ID | Context | Best For |
|---|---|---|---|
| `Llama3_3_70bVersatile` | `llama-3.3-70b-versatile` | 128 K | General-purpose (recommended) |
| `Llama3_1_8bInstant` | `llama-3.1-8b-instant` | 128 K | Fastest, most cost-effective |
| `Llama3_1_70bVersatile` | `llama-3.1-70b-versatile` | 128 K | High-quality generation |
| `Gemma2_9bIt` | `gemma2-9b-it` | 8 K | Multilingual tasks |
| `Mixtral8x7b32768` | `mixtral-8x7b-32768` | 32 K | Long-context MoE |
| `Custom(String)` | _(any)_ | -- | Unlisted / preview models |

## Usage

`GroqChatModel` implements the `ChatModel` trait. Use `chat()` for a single response:

```rust,ignore
use synaptic::groq::{GroqChatModel, GroqConfig, GroqModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = GroqConfig::new("gsk-key", GroqModel::Llama3_3_70bVersatile);
let model = GroqChatModel::new(config, Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a concise assistant."),
    Message::human("What is Rust famous for?"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content().unwrap_or_default());
```

## Streaming

Use `stream_chat()` to receive tokens as they are generated. Groq streaming is especially useful because of the high token throughput:

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message};
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Tell me about Rust ownership in 3 sentences."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content);
}
println!();
```

## Tool Calling

Groq supports OpenAI-compatible function/tool calling. Pass tool definitions and optionally a `ToolChoice`:

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message, ToolDefinition, ToolChoice};
use serde_json::json;

let tools = vec![ToolDefinition {
    name: "get_weather".to_string(),
    description: "Get current weather for a city.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": { "city": {"type": "string"} },
        "required": ["city"]
    }),
}];

let request = ChatRequest::new(vec![
    Message::human("What is the weather in Tokyo?"),
])
.with_tools(tools)
.with_tool_choice(ToolChoice::Auto);

let response = model.chat(request).await?;
for tc in response.message.tool_calls() {
    println!("Tool: {}, Args: {}", tc.name, tc.arguments);
}
```

## Error Handling

Groq enforces rate limits per API key. The `SynapticError::RateLimit` variant is returned when the API responds with HTTP 429:

```rust,ignore
use synaptic::core::SynapticError;

match model.chat(request).await {
    Ok(response) => println!("{}", response.message.content().unwrap_or_default()),
    Err(SynapticError::RateLimit(msg)) => {
        eprintln!("Rate limited: {}", msg);
        // Back off and retry
    }
    Err(e) => return Err(e.into()),
}
```

For automatic retry with exponential backoff, wrap the model with `RetryChatModel`:

```rust,ignore
use synaptic::models::{RetryChatModel, RetryConfig};

let retry_model = RetryChatModel::new(model, RetryConfig::default());
```

## Configuration Reference

### GroqConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | `String` | required | Groq API key (`gsk-...`) |
| `model` | `String` | from enum | API model identifier |
| `max_tokens` | `Option<u32>` | `None` | Maximum tokens to generate |
| `temperature` | `Option<f64>` | `None` | Sampling temperature (0.0-2.0) |
| `top_p` | `Option<f64>` | `None` | Nucleus sampling threshold |
| `stop` | `Option<Vec<String>>` | `None` | Stop sequences |
| `seed` | `Option<u64>` | `None` | Seed for reproducible output |
