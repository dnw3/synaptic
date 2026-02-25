# Groq

[Groq](https://groq.com/) 利用其专有的 LPU（语言处理单元）硬件，提供超高速的 LLM 推理服务。响应速度通常超过每秒 500 个 token，使 Groq 非常适合实时应用、交互式 Agent 和对延迟敏感的流水线。

Groq API 与 OpenAI API 格式完全兼容。Groq 作为 `synaptic-openai` 内的兼容子模块提供，无需单独的 crate。

## 设置

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

前往 [console.groq.com](https://console.groq.com/) 注册并获取 API 密钥。密钥以 `gsk-` 开头。

## 配置

```rust,ignore
use synaptic::openai::compat::groq::{self, GroqModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = groq::chat_model("gsk-your-api-key", GroqModel::Llama3_3_70bVersatile.to_string(), Arc::new(HttpBackend::new()));
```

### 构建器方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

```rust,ignore
use synaptic::openai::compat::groq::{self, GroqModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = groq::config("gsk-key", GroqModel::Llama3_3_70bVersatile.to_string())
    .with_temperature(0.7)
    .with_max_tokens(2048)
    .with_top_p(0.9);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

如需使用 `GroqModel` 中未列出的模型，直接传入字符串：

```rust,ignore
let model = groq::chat_model("gsk-key", "llama-3.1-405b", Arc::new(HttpBackend::new()));
```

## 可用模型

| 枚举变体 | API 模型 ID | 上下文长度 | 适用场景 |
|---|---|---|---|
| `Llama3_3_70bVersatile` | `llama-3.3-70b-versatile` | 128 K | 通用场景（推荐） |
| `Llama3_1_8bInstant` | `llama-3.1-8b-instant` | 128 K | 最快、最具性价比 |
| `Llama3_1_70bVersatile` | `llama-3.1-70b-versatile` | 128 K | 高质量生成 |
| `Gemma2_9bIt` | `gemma2-9b-it` | 8 K | 多语言任务 |
| `Mixtral8x7b32768` | `mixtral-8x7b-32768` | 32 K | 长上下文 MoE |
| `Custom(String)` | _(任意)_ | -- | 未列出的/预览模型 |

## 使用方法

`chat_model()` 返回的模型实现了 `ChatModel` trait。使用 `chat()` 获取单次响应：

```rust,ignore
use synaptic::openai::compat::groq::{self, GroqModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = groq::chat_model("gsk-key", GroqModel::Llama3_3_70bVersatile.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a concise assistant."),
    Message::human("What is Rust famous for?"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content().unwrap_or_default());
```

## 流式输出

使用 `stream_chat()` 实时接收生成的 token。得益于 Groq 的高吞吐量，流式输出尤为实用：

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message};
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Tell me about Rust ownership in 3 sentences."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content);
}
println!();
```

## 工具调用

Groq 支持 OpenAI 兼容的函数/工具调用。传入工具定义并可选地指定 `ToolChoice`：

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message, ToolDefinition, ToolChoice};
use serde_json::json;

let tools = vec![ToolDefinition {
    name: "get_weather".to_string(),
    description: "Get current weather for a city.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": { "city": {"type": "string"} },
        "required": ["city"]
    }),
}];

let request = ChatRequest::new(vec![
    Message::human("What is the weather in Tokyo?"),
])
.with_tools(tools)
.with_tool_choice(ToolChoice::Auto);

let response = model.chat(request).await?;
for tc in response.message.tool_calls() {
    println!("Tool: {}, Args: {}", tc.name, tc.arguments);
}
```

## 错误处理

Groq 对每个 API 密钥施加速率限制。当 API 返回 HTTP 429 时，会返回 `SynapticError::RateLimit` 错误变体：

```rust,ignore
use synaptic::core::SynapticError;

match model.chat(request).await {
    Ok(response) => println!("{}", response.message.content().unwrap_or_default()),
    Err(SynapticError::RateLimit(msg)) => {
        eprintln!("Rate limited: {}", msg);
    }
    Err(e) => return Err(e.into()),
}
```

如需自动重试，可使用 `RetryChatModel` 包装模型：

```rust,ignore
use synaptic::models::{RetryChatModel, RetryConfig};

let retry_model = RetryChatModel::new(model, RetryConfig::default());
```

## 配置参考

所有配置通过 `OpenAiConfig` 构建器方法完成。完整参考请见 [OpenAI 兼容 Provider](openai-compatible.md) 页面。

| 方法 | 说明 |
|------|------|
| `.with_temperature(f64)` | 采样温度（0.0-2.0） |
| `.with_max_tokens(u32)` | 最大生成 token 数 |
| `.with_top_p(f64)` | 核采样阈值 |
| `.with_stop(Vec<String>)` | 停止序列 |
| `.with_seed(u64)` | 可复现输出的随机种子 |
