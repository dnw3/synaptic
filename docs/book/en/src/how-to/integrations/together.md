# Together AI

[Together AI](https://www.together.ai/) provides access to leading open-source models (Llama, DeepSeek, Qwen, Mixtral) via an OpenAI-compatible API. It offers serverless inference at competitive prices, making it ideal for production workloads that require state-of-the-art open models.

Together AI is available as a compatibility submodule inside `synaptic-models`. No separate crate is needed.

## Setup

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

Sign up at [api.together.xyz](https://api.together.xyz/) to obtain an API key.

## Configuration

```rust,ignore
use synaptic::openai::compat::together::{self, TogetherModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = together::chat_model("your-api-key", TogetherModel::Llama3_3_70bInstructTurbo.to_string(), Arc::new(HttpBackend::new()));
```

### Builder methods

Use `OpenAiConfig` builder methods for customization:

```rust,ignore
use synaptic::openai::compat::together::{self, TogetherModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = together::config("your-api-key", TogetherModel::Llama3_3_70bInstructTurbo.to_string())
    .with_temperature(0.7)
    .with_max_tokens(2048)
    .with_top_p(0.9)
    .with_stop(vec!["</s>".to_string()]);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

For unlisted models, pass a string directly:

```rust,ignore
let model = together::chat_model("your-api-key", "custom-org/custom-model-v1", Arc::new(HttpBackend::new()));
```

## Available Models

| Enum Variant | API Model ID | Best For |
|---|---|---|
| `Llama3_3_70bInstructTurbo` | `meta-llama/Llama-3.3-70B-Instruct-Turbo` | General purpose (recommended) |
| `Llama3_1_8bInstructTurbo` | `meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo` | Fast, cost-effective |
| `Llama3_1_405bInstructTurbo` | `meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo` | Maximum quality |
| `DeepSeekR1` | `deepseek-ai/DeepSeek-R1` | Reasoning tasks |
| `Qwen2_5_72bInstructTurbo` | `Qwen/Qwen2.5-72B-Instruct-Turbo` | Multilingual |
| `Mixtral8x7bInstruct` | `mistralai/Mixtral-8x7B-Instruct-v0.1` | Long-context MoE |
| `Custom(String)` | _(any)_ | Unlisted / preview models |

## Usage

```rust,ignore
use synaptic::openai::compat::together::{self, TogetherModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = together::chat_model("your-api-key", TogetherModel::Llama3_3_70bInstructTurbo.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a concise assistant."),
    Message::human("What is Rust famous for?"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## Streaming

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Explain Rust's ownership model in 3 sentences."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content);
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
