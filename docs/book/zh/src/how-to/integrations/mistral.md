# Mistral AI

[Mistral AI](https://mistral.ai/) 提供最先进的开源和商业语言模型，具备出色的多语言支持和强大的函数调用能力。Mistral API 与 OpenAI API 格式完全兼容。

`synaptic-mistral` crate 对 `synaptic-openai` 进行封装，预设了 Mistral 的 base URL，并提供类型安全的 `MistralModel` 枚举。同时，它还提供了 `mistral_embeddings` 辅助函数用于访问 Mistral 嵌入向量 API。

## 设置

在 `Cargo.toml` 中添加 `mistral` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["mistral"] }
```

前往 [console.mistral.ai](https://console.mistral.ai/) 获取 API 密钥。

## 配置

使用 API 密钥和 `MistralModel` 变体创建 `MistralConfig`：

```rust,ignore
use synaptic::mistral::{MistralChatModel, MistralConfig, MistralModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = MistralConfig::new("your-api-key", MistralModel::MistralLargeLatest);
let model = MistralChatModel::new(config, Arc::new(HttpBackend::new()));
```

### 构建器方法

`MistralConfig` 支持与其他 Provider 相同的流式构建器模式：

```rust,ignore
let config = MistralConfig::new("key", MistralModel::MistralLargeLatest)
    .with_temperature(0.7)
    .with_max_tokens(4096)
    .with_top_p(0.95)
    .with_seed(123);
```

## 可用模型

| 枚举变体 | API 模型 ID | 上下文长度 | 适用场景 |
|---|---|---|---|
| `MistralLargeLatest` | `mistral-large-latest` | 128 K | 最强能力，复杂推理 |
| `MistralSmallLatest` | `mistral-small-latest` | 32 K | 性能与成本的平衡 |
| `OpenMistralNemo` | `open-mistral-nemo` | 128 K | 开源，强多语言支持 |
| `CodestralLatest` | `codestral-latest` | 32 K | 代码生成与补全 |
| `Custom(String)` | _(任意)_ | -- | 未列出的/预览模型 |

## 使用方法

`MistralChatModel` 实现了 `ChatModel` trait：

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

## 流式输出

使用 `stream_chat()` 逐步接收生成的 token：

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

## 工具调用

Mistral 模型具备强大的函数调用能力：

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

## 嵌入向量

Mistral 提供与聊天 API 相同 base URL 的嵌入向量 API。使用 `mistral_embeddings` 辅助函数：

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

// 嵌入单个查询
let vector = embeddings.embed_query("What is ownership in Rust?").await?;
println!("维度: {}", vector.len()); // 1024

// 批量嵌入文档
let docs = ["Rust is safe.", "Rust is fast.", "Rust is fun."];
let vectors = embeddings.embed_documents(&docs).await?;
println!("已嵌入 {} 个文档", vectors.len());
```

## 错误处理

当 API 返回 HTTP 429 时，会返回 `SynapticError::RateLimit` 错误变体：

```rust,ignore
use synaptic::core::SynapticError;

match model.chat(request).await {
    Ok(response) => println!("{}", response.message.content().unwrap_or_default()),
    Err(SynapticError::RateLimit(msg)) => eprintln!("Rate limited: {}", msg),
    Err(e) => return Err(e.into()),
}
```

## 配置参考

### MistralConfig

| 字段 | 类型 | 默认值 | 说明 |
|-------|------|---------|-------------|
| `api_key` | `String` | 必填 | Mistral AI API 密钥 |
| `model` | `String` | 来自枚举 | API 模型标识符 |
| `max_tokens` | `Option<u32>` | `None` | 最大生成 token 数 |
| `temperature` | `Option<f64>` | `None` | 采样温度（0.0-1.0） |
| `top_p` | `Option<f64>` | `None` | 核采样阈值 |
| `stop` | `Option<Vec<String>>` | `None` | 停止序列 |
| `seed` | `Option<u64>` | `None` | 可复现输出的随机种子 |
