# Fireworks AI

[Fireworks AI](https://fireworks.ai/) 提供最快的开源模型推理，主流模型首 token 延迟低于 100ms。采用 OpenAI 兼容 API，支持 Llama、DeepSeek、Qwen 等主流开源模型。

Fireworks AI 作为 `synaptic-models` 内的兼容子模块提供，无需单独的 crate。

## 安装

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

在 [fireworks.ai](https://fireworks.ai/) 注册以获取 API 密钥（以 `fw-` 开头）。

## 配置

```rust,ignore
use synaptic::openai::compat::fireworks::{self, FireworksModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = fireworks::chat_model("fw-your-api-key", FireworksModel::Llama3_1_70bInstruct.to_string(), Arc::new(HttpBackend::new()));
```

### Builder 方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

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

## 可用模型

| 枚举变体 | API 模型 ID | 适用场景 |
|---|---|---|
| `Llama3_1_70bInstruct` | `accounts/fireworks/models/llama-v3p1-70b-instruct` | 通用（推荐） |
| `Llama3_1_8bInstruct` | `accounts/fireworks/models/llama-v3p1-8b-instruct` | 最快、低成本 |
| `DeepSeekR1` | `accounts/fireworks/models/deepseek-r1` | 推理任务 |
| `Qwen2_5_72bInstruct` | `accounts/fireworks/models/qwen2p5-72b-instruct` | 多语言 |
| `Custom(String)` | _(任意)_ | 未列出/预览模型 |

## 使用示例

```rust,ignore
use synaptic::openai::compat::fireworks::{self, FireworksModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = fireworks::chat_model("fw-your-api-key", FireworksModel::Llama3_1_70bInstruct.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("你是一个有用的助手。"),
    Message::human("解释 Rust 中 async 与多线程的区别。"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## 流式输出

```rust,ignore
use futures::StreamExt;

let mut stream = model.stream_chat(ChatRequest::new(vec![
    Message::human("写一首关于 Rust 编程的俳句。"),
]));
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.content);
}
println!();
```

## 配置参考

所有配置通过 `OpenAiConfig` 构建器方法完成。完整参考请见 [OpenAI 兼容 Provider](openai-compatible.md) 页面。

| 方法 | 说明 |
|------|------|
| `.with_temperature(f64)` | 采样温度（0.0-2.0） |
| `.with_max_tokens(u32)` | 最大生成 token 数 |
| `.with_top_p(f64)` | 核采样阈值 |
| `.with_stop(Vec<String>)` | 停止序列 |
