# Mistral AI

[Mistral AI](https://mistral.ai/) offers state-of-the-art open and proprietary language models with excellent multilingual support and strong function-calling capabilities. The Mistral API is fully compatible with the OpenAI API format.

The `synaptic-mistral` crate wraps `synaptic-openai` with the Mistral base URL preset and a type-safe `MistralModel` enum. It also provides a `mistral_embeddings` helper for the Mistral embeddings endpoint.

## Setup

Add the `mistral` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["mistral"] }
```

Obtain an API key from [console.mistral.ai](https://console.mistral.ai/).

## Configuration

Create a `MistralConfig` with your API key and a `MistralModel` variant:

```rust,ignore
use synaptic::mistral::{MistralChatModel, MistralConfig, MistralModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = MistralConfig::new("your-api-key", MistralModel::MistralLargeLatest);
let model = MistralChatModel::new(config, Arc::new(HttpBackend::new()));
```

### Builder methods

`MistralConfig` supports the same fluent builder pattern as other providers:

```rust,ignore
let config = MistralConfig::new("key", MistralModel::MistralLargeLatest)
    .with_temperature(0.7)
    .with_max_tokens(4096)
    .with_top_p(0.95)
    .with_seed(123);
```

For unlisted models:

```rust,ignore
let config = MistralConfig::new_custom("key", "mistral-large-2411");
```

## Available Models

| Enum Variant | API Model ID | Context | Best For |
|---|---|---|---|
| `MistralLargeLatest` | `mistral-large-latest` | 128 K | Most capable, complex reasoning |
| `MistralSmallLatest` | `mistral-small-latest` | 32 K | Balanced performance and cost |
| `OpenMistralNemo` | `open-mistral-nemo` | 128 K | Open-source, strong multilingual |
| `CodestralLatest` | `codestral-latest` | 32 K | Code generation and completion |
| `Custom(String)` | _(any)_ | -- | Unlisted / preview models |

## Usage

`MistralChatModel` implements the `ChatModel` trait:

```rust,ignore
use synaptic::mistral::{MistralChatModel, MistralConfig, MistralModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = MistralConfig::new("key", MistralModel::MistralLargeLatest);
let model = MistralChatModel::new(config, Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec\![
    Message::system("You are a helpful multilingual assistant."),
    Message::human("Bonjour\! Explain Rust ownership in one sentence."),
]);

let response = model.chat(request).await?;
println\!("{}", response.message.content().unwrap_or_default());
```

## Streaming

Use `stream_chat()` to receive tokens incrementally:

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Write a haiku about distributed systems."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.content);
}
println!();
```

## Tool Calling

Mistral models have strong function-calling capabilities:

```rust,ignore
use synaptic::core::{ChatRequest, Message, ToolDefinition, ToolChoice};
use serde_json::json;

let tools = vec![ToolDefinition {
    name: "search_documents".to_string(),
    description: "Search a document database.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": { "query": {"type": "string"} },
        "required": ["query"]
    }),
}];

let request = ChatRequest::new(vec![Message::human("Find documents about Rust async.")])
    .with_tools(tools)
    .with_tool_choice(ToolChoice::Auto);

let response = model.chat(request).await?;
for tc in response.message.tool_calls() {
    println!("Tool: {}, Args: {}", tc.name, tc.arguments);
}
```

## Embeddings

Mistral provides an embeddings API through the same base URL. Use the `mistral_embeddings` helper function:

```rust,ignore
use synaptic::mistral::mistral_embeddings;
use synaptic::models::HttpBackend;
use synaptic::core::Embeddings;
use std::sync::Arc;

let embeddings = mistral_embeddings(
    "your-api-key",
    "mistral-embed",
    Arc::new(HttpBackend::new()),
);

// Embed a single query
let vector = embeddings.embed_query("What is ownership in Rust?").await?;
println!("Dimension: {}", vector.len()); // 1024

// Embed multiple documents for indexing
let docs = ["Rust is safe.", "Rust is fast.", "Rust is fun."];
let vectors = embeddings.embed_documents(&docs).await?;
println!("Embedded {} documents", vectors.len());
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

### MistralConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | `String` | required | Mistral AI API key |
| `model` | `String` | from enum | API model identifier |
| `max_tokens` | `Option<u32>` | `None` | Maximum tokens to generate |
| `temperature` | `Option<f64>` | `None` | Sampling temperature (0.0-1.0) |
| `top_p` | `Option<f64>` | `None` | Nucleus sampling threshold |
| `stop` | `Option<Vec<String>>` | `None` | Stop sequences |
| `seed` | `Option<u64>` | `None` | Seed for reproducible output |
