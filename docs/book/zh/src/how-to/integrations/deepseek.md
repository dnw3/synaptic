# DeepSeek

[DeepSeek](https://deepseek.com/) 以极低的成本提供强大的语言和推理模型。DeepSeek 模型的价格通常比同类商业模型（如 GPT-4o）低 90% 以上，同时在许多基准测试中表现相当甚至更优。

DeepSeek API 与 OpenAI API 格式完全兼容。DeepSeek 作为 `synaptic-openai` 内的兼容子模块提供，无需单独的 crate。

## 设置

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

前往 [platform.deepseek.com](https://platform.deepseek.com/) 获取 API 密钥。密钥以 `sk-` 开头。

## 配置

```rust,ignore
use synaptic::openai::compat::deepseek::{self, DeepSeekModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = deepseek::chat_model("sk-your-api-key", DeepSeekModel::DeepSeekChat.to_string(), Arc::new(HttpBackend::new()));
```

### 构建器方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

```rust,ignore
use synaptic::openai::compat::deepseek::{self, DeepSeekModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = deepseek::config("sk-key", DeepSeekModel::DeepSeekChat.to_string())
    .with_temperature(0.3)
    .with_max_tokens(4096)
    .with_top_p(0.9);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

使用未列出的模型，直接传入字符串：

```rust,ignore
let model = deepseek::chat_model("sk-key", "deepseek-chat", Arc::new(HttpBackend::new()));
```

## 可用模型

| 枚举变体 | API 模型 ID | 上下文长度 | 适用场景 |
|---|---|---|---|
| `DeepSeekChat` | `deepseek-chat` | 64 K | 通用场景，极低成本 |
| `DeepSeekReasoner` | `deepseek-reasoner` | 64 K | 链式思维推理（R1） |
| `DeepSeekCoderV2` | `deepseek-coder-v2` | 128 K | 代码生成与分析 |
| `Custom(String)` | _(任意)_ | -- | 未列出的/预览模型 |

### 成本对比

DeepSeek-V3（`DeepSeekChat`）的定价约为每百万输出 token 0.27 美元，而 GPT-4o 为每百万 token 15 美元。这使 DeepSeek 成为高吞吐量场景和大规模实验的理想选择。

### DeepSeek-R1 推理模型

`DeepSeekReasoner` 模型（R1）采用链式思维推理来解决复杂问题。它会在给出最终答案之前，在 `<think>` 块中展示推理过程，特别适合数学、编程挑战和逻辑推理任务。

## 使用方法

`chat_model()` 返回的模型实现了 `ChatModel` trait：

```rust,ignore
use synaptic::openai::compat::deepseek::{self, DeepSeekModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = deepseek::chat_model("sk-key", DeepSeekModel::DeepSeekChat.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("You are a concise technical assistant."),
    Message::human("Explain Rust borrow checker in one sentence."),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content().unwrap_or_default());
```

## 流式输出

使用 `stream_chat()` 逐步接收生成的 token：

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("Write a Rust function that parses JSON."),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.content);
}
println!();
```

## 工具调用

DeepSeek-V3 支持 OpenAI 兼容的工具调用：

```rust,ignore
use synaptic::core::{ChatRequest, Message, ToolDefinition, ToolChoice};
use serde_json::json;

let tools = vec![ToolDefinition {
    name: "calculate".to_string(),
    description: "Evaluate a mathematical expression.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": { "expression": {"type": "string"} },
        "required": ["expression"]
    }),
}];

let request = ChatRequest::new(vec![Message::human("What is 42 * 1337?")])
    .with_tools(tools)
    .with_tool_choice(ToolChoice::Auto);

let response = model.chat(request).await?;
for tc in response.message.tool_calls() {
    println!("Tool: {}, Args: {}", tc.name, tc.arguments);
}
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
| `.with_temperature(f64)` | 采样温度（0.0-2.0） |
| `.with_max_tokens(u32)` | 最大生成 token 数 |
| `.with_top_p(f64)` | 核采样阈值 |
| `.with_stop(Vec<String>)` | 停止序列 |
| `.with_seed(u64)` | 可复现输出的随机种子 |
