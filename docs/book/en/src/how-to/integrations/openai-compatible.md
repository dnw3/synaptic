# OpenAI-Compatible Providers

Many LLM providers expose an OpenAI-compatible API. Synaptic ships convenience constructors for eleven popular providers as submodules of `synaptic::openai::compat`, so you can connect without building configuration by hand.

## Setup

Add the `openai` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

All OpenAI-compatible providers use the `synaptic-models` crate under the hood, so only the `openai` feature is required.

## Supported Providers

The `synaptic::openai::compat` module provides a submodule per provider, each with two functions:

- `config(api_key, model)` -- returns an `OpenAiConfig` pre-configured with the correct base URL.
- `chat_model(api_key, model, backend)` -- returns a ready-to-use `OpenAiChatModel`.

Some providers also offer embeddings variants.

| Provider | Submodule | Embeddings? |
|----------|-----------|-------------|
| Groq | `compat::groq` | No |
| DeepSeek | `compat::deepseek` | No |
| Fireworks | `compat::fireworks` | No |
| Together | `compat::together` | No |
| xAI | `compat::xai` | No |
| Perplexity | `compat::perplexity` | No |
| MistralAI | `compat::mistral` | Yes |
| HuggingFace | `compat::huggingface` | Yes |
| Cohere | `compat::cohere` | Yes |
| OpenRouter | `compat::openrouter` | No |

## Usage

### Chat model

```rust,ignore
use std::sync::Arc;
use synaptic::openai::compat::{groq, deepseek};
use synaptic::models::HttpBackend;
use synaptic::core::{ChatModel, ChatRequest, Message};

let backend = Arc::new(HttpBackend::new());

// Groq
let model = groq::chat_model("gsk-...", "llama-3.3-70b-versatile", backend.clone());
let request = ChatRequest::new(vec![Message::human("Hello from Groq!")]);
let response = model.chat(&request).await?;

// DeepSeek
let model = deepseek::chat_model("sk-...", "deepseek-chat", backend.clone());
let response = model.chat(&request).await?;
```

### Config-first approach

If you need to customize the config further before creating the model:

```rust,ignore
use std::sync::Arc;
use synaptic::openai::compat::fireworks;
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;

let config = fireworks::config("fw-...", "accounts/fireworks/models/llama-v3p1-70b-instruct")
    .with_temperature(0.7)
    .with_max_tokens(2048);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

### Type-safe model enums

Each provider submodule exports a model enum with common variants:

```rust,ignore
use synaptic::openai::compat::groq::{self, GroqModel};

let model = groq::chat_model("gsk-...", GroqModel::Llama3_3_70bVersatile.to_string(), backend.clone());
```

### Embeddings

Providers that support embeddings have `embeddings_config` and `embeddings` functions:

```rust,ignore
use std::sync::Arc;
use synaptic::openai::compat::{mistral, cohere, huggingface};
use synaptic::models::HttpBackend;
use synaptic::core::Embeddings;

let backend = Arc::new(HttpBackend::new());

// MistralAI embeddings
let embeddings = mistral::embeddings("sk-...", "mistral-embed", backend.clone());
let vectors = embeddings.embed_documents(&["Hello world"]).await?;

// Cohere embeddings
let embeddings = cohere::embeddings("co-...", "embed-english-v3.0", backend.clone());

// HuggingFace embeddings
let embeddings = huggingface::embeddings("hf_...", "BAAI/bge-small-en-v1.5", backend.clone());
```

## Unlisted providers

Any provider that exposes an OpenAI-compatible API can be used by setting a custom base URL on `OpenAiConfig`:

```rust,ignore
use std::sync::Arc;
use synaptic::openai::{OpenAiConfig, OpenAiChatModel};
use synaptic::models::HttpBackend;

let config = OpenAiConfig::new("your-api-key", "model-name")
    .with_base_url("https://api.example.com/v1");

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

This works for any service that accepts the OpenAI chat completions request format at `{base_url}/chat/completions`.

## Streaming

All OpenAI-compatible models support streaming. Use `stream_chat()` just like you would with the standard `OpenAiChatModel`:

```rust,ignore
use futures::StreamExt;
use synaptic::core::{ChatModel, ChatRequest, Message};

let request = ChatRequest::new(vec![Message::human("Tell me a story")]);
let mut stream = model.stream_chat(&request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(text) = &chunk.content {
        print!("{}", text);
    }
}
```

## Provider reference

| Provider | Base URL | Env variable (convention) |
|----------|----------|--------------------------|
| Groq | `https://api.groq.com/openai/v1` | `GROQ_API_KEY` |
| DeepSeek | `https://api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| Fireworks | `https://api.fireworks.ai/inference/v1` | `FIREWORKS_API_KEY` |
| Together | `https://api.together.xyz/v1` | `TOGETHER_API_KEY` |
| xAI | `https://api.x.ai/v1` | `XAI_API_KEY` |
| Perplexity | `https://api.perplexity.ai` | `PERPLEXITY_API_KEY` |
| MistralAI | `https://api.mistral.ai/v1` | `MISTRAL_API_KEY` |
| HuggingFace | `https://api-inference.huggingface.co/v1` | `HUGGINGFACE_API_KEY` |
| Cohere | `https://api.cohere.com/v1` | `CO_API_KEY` |
| OpenRouter | `https://openrouter.ai/api/v1` | `OPENROUTER_API_KEY` |
