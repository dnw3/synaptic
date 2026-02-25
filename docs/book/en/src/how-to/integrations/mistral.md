# Mistral AI

[Mistral AI](https://mistral.ai/) offers state-of-the-art open and proprietary language models with excellent multilingual support and strong function-calling capabilities. The Mistral API is fully compatible with the OpenAI API format.

Mistral AI is available as a compatibility submodule inside `synaptic-openai`. No separate crate is needed. The submodule also provides an `embeddings` helper for the Mistral embeddings endpoint.

## Setup

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

Obtain an API key from [console.mistral.ai](https://console.mistral.ai/).

## Configuration

```rust,ignore
use synaptic::openai::compat::mistral::{self, MistralModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = mistral::chat_model("your-api-key", MistralModel::MistralLargeLatest.to_string(), Arc::new(HttpBackend::new()));
```

### Builder methods

Use `OpenAiConfig` builder methods for customization:

```rust,ignore
use synaptic::openai::compat::mistral::{self, MistralModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = mistral::config("key", MistralModel::MistralLargeLatest.to_string())
    .with_temperature(0.7)
    .with_max_tokens(4096)
    .with_top_p(0.95);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

For unlisted models, pass a string directly:

```rust,ignore
let model = mistral::chat_model("key", "mistral-large-2411", Arc::new(HttpBackend::new()));
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

The model returned by `chat_model()` implements the `ChatModel` trait:

```rust,ignore
use synaptic::openai::compat::mistral::{self, MistralModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = mistral::chat_model("key", MistralModel::MistralLargeLatest.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a helpful multilingual assistant."),
    Message::human("Bonjour! Explain Rust ownership in one sentence."),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content().unwrap_or_default());
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

Mistral provides an embeddings API through the same base URL. Use the `embeddings` helper function:

```rust,ignore
use synaptic::openai::compat::mistral;
use synaptic::models::HttpBackend;
use synaptic::core::Embeddings;
use std::sync::Arc;

let embeddings = mistral::embeddings(
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

All configuration is done through `OpenAiConfig` builder methods. See the [OpenAI-Compatible Providers](openai-compatible.md) page for the full reference.

| Method | Description |
|--------|-------------|
| `.with_temperature(f64)` | Sampling temperature (0.0-1.0) |
| `.with_max_tokens(u32)` | Maximum tokens to generate |
| `.with_top_p(f64)` | Nucleus sampling threshold |
| `.with_stop(Vec<String>)` | Stop sequences |
| `.with_seed(u64)` | Seed for reproducible output |
