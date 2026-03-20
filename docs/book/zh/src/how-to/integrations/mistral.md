# Mistral AI

[Mistral AI](https://mistral.ai/) 提供最先进的开源和商业语言模型，具备出色的多语言支持和强大的函数调用能力。Mistral API 与 OpenAI API 格式完全兼容。

Mistral AI 作为 `synaptic-models` 内的兼容子模块提供，无需单独的 crate。该子模块还提供 `embeddings` 辅助函数用于访问 Mistral 嵌入向量 API。

## 设置

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

前往 [console.mistral.ai](https://console.mistral.ai/) 获取 API 密钥。

## 配置

```rust,ignore
use synaptic::openai::compat::mistral::{self, MistralModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = mistral::chat_model("your-api-key", MistralModel::MistralLargeLatest.to_string(), Arc::new(HttpBackend::new()));
```

### 构建器方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

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

使用未列出的模型，直接传入字符串：

```rust,ignore
let model = mistral::chat_model("key", "mistral-large-2411", Arc::new(HttpBackend::new()));
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

`chat_model()` 返回的模型实现了 `ChatModel` trait：

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

Mistral 提供与聊天 API 相同 base URL 的嵌入向量 API。使用 `embeddings` 辅助函数：

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

所有配置通过 `OpenAiConfig` 构建器方法完成。完整参考请见 [OpenAI 兼容 Provider](openai-compatible.md) 页面。

| 方法 | 说明 |
|------|------|
| `.with_temperature(f64)` | 采样温度（0.0-1.0） |
| `.with_max_tokens(u32)` | 最大生成 token 数 |
| `.with_top_p(f64)` | 核采样阈值 |
| `.with_stop(Vec<String>)` | 停止序列 |
| `.with_seed(u64)` | 可复现输出的随机种子 |
