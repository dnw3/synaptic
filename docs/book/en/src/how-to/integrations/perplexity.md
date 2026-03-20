# Perplexity AI

[Perplexity AI](https://www.perplexity.ai/) provides online search-augmented language models through its Sonar model family. Unlike traditional LLMs, Sonar models access real-time web information and return cited sources, making them ideal for factual queries and research tasks.

Perplexity AI is available as a compatibility submodule inside `synaptic-models`. No separate crate is needed.

## Setup

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

Sign up at [perplexity.ai](https://www.perplexity.ai/) to obtain an API key (prefixed with `pplx-`).

## Configuration

```rust,ignore
use synaptic::openai::compat::perplexity::{self, PerplexityModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = perplexity::chat_model("pplx-your-api-key", PerplexityModel::SonarLarge.to_string(), Arc::new(HttpBackend::new()));
```

### Builder methods

Use `OpenAiConfig` builder methods for customization:

```rust,ignore
use synaptic::openai::compat::perplexity::{self, PerplexityModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = perplexity::config("pplx-your-api-key", PerplexityModel::SonarLarge.to_string())
    .with_temperature(0.2)
    .with_max_tokens(1024);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

## Available Models

| Enum Variant | API Model ID | Best For |
|---|---|---|
| `SonarLarge` | `sonar-large-online` | General web search (recommended) |
| `SonarSmall` | `sonar-small-online` | Fast, cost-effective web search |
| `SonarHuge` | `sonar-huge-online` | Maximum quality web search |
| `SonarReasoningPro` | `sonar-reasoning-pro` | Complex reasoning with citations |
| `Custom(String)` | _(any)_ | Preview models |

## Usage

```rust,ignore
use synaptic::openai::compat::perplexity::{self, PerplexityModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = perplexity::chat_model("pplx-your-api-key", PerplexityModel::SonarLarge.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("Be precise and concise. Cite your sources."),
    Message::human("What is the current state of Rust adoption in systems programming?"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## Streaming

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("What are the latest developments in LLM research?"),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.content);
}
println!();
```

## Error Handling

```rust,ignore
use synaptic::core::SynapticError;

match model.chat(request).await {
    Ok(response) => println!("{}", response.message.content()),
    Err(SynapticError::RateLimit(msg)) => eprintln!("Rate limited: {}", msg),
    Err(e) => return Err(e.into()),
}
```

## Configuration Reference

All configuration is done through `OpenAiConfig` builder methods. See the [OpenAI-Compatible Providers](openai-compatible.md) page for the full reference.

| Method | Description |
|--------|-------------|
| `.with_temperature(f64)` | Sampling temperature (0.0-2.0) |
| `.with_max_tokens(u32)` | Maximum tokens to generate |
| `.with_top_p(f64)` | Nucleus sampling threshold |
| `.with_stop(Vec<String>)` | Stop sequences |
