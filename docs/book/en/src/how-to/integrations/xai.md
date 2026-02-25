# xAI Grok

[xAI](https://x.ai/) develops the Grok family of LLMs known for their real-time reasoning capabilities and integration with X (Twitter) data. The Grok API is OpenAI-compatible.

xAI is available as a compatibility submodule inside `synaptic-openai`. No separate crate is needed.

## Setup

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

Sign up at [x.ai](https://x.ai/) to obtain an API key.

## Configuration

```rust,ignore
use synaptic::openai::compat::xai::{self, XaiModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = xai::chat_model("xai-your-api-key", XaiModel::Grok2Latest.to_string(), Arc::new(HttpBackend::new()));
```

### Builder methods

Use `OpenAiConfig` builder methods for customization:

```rust,ignore
use synaptic::openai::compat::xai::{self, XaiModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = xai::config("xai-your-api-key", XaiModel::Grok2Latest.to_string())
    .with_temperature(0.7)
    .with_max_tokens(8192);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

## Available Models

| Enum Variant | API Model ID | Best For |
|---|---|---|
| `Grok2Latest` | `grok-2-latest` | General purpose (recommended) |
| `Grok2Mini` | `grok-2-mini` | Fast, cost-effective |
| `GrokBeta` | `grok-beta` | Legacy compatibility |
| `Custom(String)` | _(any)_ | Preview models |

## Usage

```rust,ignore
use synaptic::openai::compat::xai::{self, XaiModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = xai::chat_model("xai-your-api-key", XaiModel::Grok2Latest.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are Grok, a helpful AI with wit and humor."),
    Message::human("What's happening in AI today?"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## Streaming

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Give me a quick summary of today's AI trends."),
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
