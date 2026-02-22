# Groq

[Groq](https://groq.com/) 利用其专有的 LPU（语言处理单元）硬件，提供超高速的 LLM 推理服务。响应速度通常超过每秒 500 个 token，使 Groq 非常适合实时应用、交互式 Agent 和对延迟敏感的流水线。

Groq API 与 OpenAI API 格式完全兼容。`synaptic-groq` crate 对 `synaptic-openai` 进行封装，预设了 Groq 的 base URL，并提供类型安全的模型名称枚举。

## 设置

在 `Cargo.toml` 中添加 `groq` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["groq"] }
```

前往 [console.groq.com](https://console.groq.com/) 注册并获取 API 密钥。密钥以 `gsk-` 开头。

## 配置

使用 API 密钥和 `GroqModel` 变体创建 `GroqConfig`：

```rust,ignore
use synaptic::groq::{GroqChatModel, GroqConfig, GroqModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = GroqConfig::new("gsk-your-api-key", GroqModel::Llama3_3_70bVersatile);
let model = GroqChatModel::new(config, Arc::new(HttpBackend::new()));
```

### 构建器方法

`GroqConfig` 提供流式构建器用于设置可选参数：

```rust,ignore
let config = GroqConfig::new("gsk-key", GroqModel::Llama3_3_70bVersatile)
    .with_temperature(0.7)
    .with_max_tokens(2048)
    .with_top_p(0.9)
    .with_seed(42)
    .with_stop(vec\!["<|end|>".to_string()]);
```

如需使用 `GroqModel` 中未列出的模型，可使用自定义变体：

```rust,ignore
let config = GroqConfig::new_custom("gsk-key", "llama-3.1-405b");
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

`GroqChatModel` 实现了 `ChatModel` trait。使用 `chat()` 获取单次响应：

```rust,ignore
use synaptic::groq::{GroqChatModel, GroqConfig, GroqModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = GroqConfig::new("gsk-key", GroqModel::Llama3_3_70bVersatile);
let model = GroqChatModel::new(config, Arc::new(HttpBackend::new()));

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

### GroqConfig

| 字段 | 类型 | 默认值 | 说明 |
|-------|------|---------|-------------|
| `api_key` | `String` | 必填 | Groq API 密钥（`gsk-...`） |
| `model` | `String` | 来自枚举 | API 模型标识符 |
| `max_tokens` | `Option<u32>` | `None` | 最大生成 token 数 |
| `temperature` | `Option<f64>` | `None` | 采样温度（0.0-2.0） |
| `top_p` | `Option<f64>` | `None` | 核采样阈值 |
| `stop` | `Option<Vec<String>>` | `None` | 停止序列 |
| `seed` | `Option<u64>` | `None` | 可复现输出的随机种子 |
