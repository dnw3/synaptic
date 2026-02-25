# OpenAI 兼容 Provider

许多 LLM 提供商暴露了与 OpenAI 兼容的 API。Synaptic 为十一个常用提供商提供了 `synaptic::openai::compat` 下的便捷子模块，无需手动构建配置即可快速接入。

## 设置

在 `Cargo.toml` 中添加 `openai` feature：

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

所有 OpenAI 兼容的提供商底层都使用 `synaptic-openai` crate，因此只需启用 `openai` feature。

## 支持的提供商

`synaptic::openai::compat` 模块为每个提供商提供一个子模块，每个子模块包含两个函数：

- `config(api_key, model)` -- 返回预配置了正确 base URL 的 `OpenAiConfig`。
- `chat_model(api_key, model, backend)` -- 返回可直接使用的 `OpenAiChatModel`。

部分提供商还提供 embeddings 变体。

| 提供商 | 子模块 | Embeddings？ |
|--------|--------|-------------|
| Groq | `compat::groq` | 否 |
| DeepSeek | `compat::deepseek` | 否 |
| Fireworks | `compat::fireworks` | 否 |
| Together | `compat::together` | 否 |
| xAI | `compat::xai` | 否 |
| Perplexity | `compat::perplexity` | 否 |
| MistralAI | `compat::mistral` | 是 |
| HuggingFace | `compat::huggingface` | 是 |
| Cohere | `compat::cohere` | 是 |
| OpenRouter | `compat::openrouter` | 否 |

## 用法

### Chat Model

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

### Config 优先方式

如果需要在创建模型前进一步自定义配置：

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

### 类型安全的模型枚举

每个提供商子模块导出一个包含常用变体的模型枚举：

```rust,ignore
use synaptic::openai::compat::groq::{self, GroqModel};

let model = groq::chat_model("gsk-...", GroqModel::Llama3_3_70bVersatile.to_string(), backend.clone());
```

### Embeddings

支持 embeddings 的提供商提供 `embeddings_config` 和 `embeddings` 函数：

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

## 未列出的提供商

任何暴露 OpenAI 兼容 API 的提供商都可以通过 `OpenAiConfig` 设置自定义 base URL 来使用：

```rust,ignore
use std::sync::Arc;
use synaptic::openai::{OpenAiConfig, OpenAiChatModel};
use synaptic::models::HttpBackend;

let config = OpenAiConfig::new("your-api-key", "model-name")
    .with_base_url("https://api.example.com/v1");

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

这适用于任何在 `{base_url}/chat/completions` 接受 OpenAI chat completions 请求格式的服务。

## 流式输出

所有 OpenAI 兼容的模型都支持流式输出。像使用标准 `OpenAiChatModel` 一样使用 `stream_chat()`：

```rust,ignore
use futures::StreamExt;
use synaptic::core::{ChatModel, ChatRequest, Message};

let request = ChatRequest::new(vec![Message::human("给我讲一个故事")]);
let mut stream = model.stream_chat(&request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(text) = &chunk.content {
        print!("{}", text);
    }
}
```

## 提供商参考

| 提供商 | Base URL | 环境变量（惯例） |
|--------|----------|-----------------|
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
