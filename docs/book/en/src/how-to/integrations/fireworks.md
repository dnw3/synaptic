# Fireworks AI

[Fireworks AI](https://fireworks.ai/) delivers the fastest open-source model inference available, with sub-100ms time-to-first-token for popular models. It uses an OpenAI-compatible API and supports Llama, DeepSeek, Qwen, and other leading open models.

Fireworks AI is available as a compatibility submodule inside `synaptic-models`. No separate crate is needed.

## Setup

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

Sign up at [fireworks.ai](https://fireworks.ai/) to obtain an API key (prefixed with `fw-`).

## Configuration

```rust,ignore
use synaptic::openai::compat::fireworks::{self, FireworksModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = fireworks::chat_model("fw-your-api-key", FireworksModel::Llama3_1_70bInstruct.to_string(), Arc::new(HttpBackend::new()));
```

### Builder methods

Use `OpenAiConfig` builder methods for customization:

```rust,ignore
use synaptic::openai::compat::fireworks::{self, FireworksModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = fireworks::config("fw-your-api-key", FireworksModel::Llama3_1_70bInstruct.to_string())
    .with_temperature(0.7)
    .with_max_tokens(4096)
    .with_top_p(0.95);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

## Available Models

| Enum Variant | API Model ID | Best For |
|---|---|---|
| `Llama3_1_70bInstruct` | `accounts/fireworks/models/llama-v3p1-70b-instruct` | General purpose (recommended) |
| `Llama3_1_8bInstruct` | `accounts/fireworks/models/llama-v3p1-8b-instruct` | Fastest, most cost-effective |
| `DeepSeekR1` | `accounts/fireworks/models/deepseek-r1` | Reasoning tasks |
| `Qwen2_5_72bInstruct` | `accounts/fireworks/models/qwen2p5-72b-instruct` | Multilingual |
| `Custom(String)` | _(any)_ | Unlisted / preview models |

## Usage

```rust,ignore
use synaptic::openai::compat::fireworks::{self, FireworksModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = fireworks::chat_model("fw-your-api-key", FireworksModel::Llama3_1_70bInstruct.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a helpful assistant."),
    Message::human("Explain the difference between async and threading in Rust."),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## Streaming

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Write a haiku about Rust programming."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.content);
}
println!();
```

## Configuration Reference

All configuration is done through `OpenAiConfig` builder methods. See the [OpenAI-Compatible Providers](openai-compatible.md) page for the full reference.

| Method | Description |
|--------|-------------|
| `.with_temperature(f64)` | Sampling temperature (0.0-2.0) |
| `.with_max_tokens(u32)` | Maximum tokens to generate |
| `.with_top_p(f64)` | Nucleus sampling threshold |
| `.with_stop(Vec<String>)` | Stop sequences |
