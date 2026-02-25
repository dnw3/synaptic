# Together AI

[Together AI](https://www.together.ai/) 通过 OpenAI 兼容 API 提供领先的开源模型访问（Llama、DeepSeek、Qwen、Mixtral）。其 Serverless 推理定价具有竞争力，适合需要前沿开源模型的生产工作负载。

Together AI 作为 `synaptic-openai` 内的兼容子模块提供，无需单独的 crate。

## 安装

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

在 [api.together.xyz](https://api.together.xyz/) 注册以获取 API 密钥。

## 配置

```rust,ignore
use synaptic::openai::compat::together::{self, TogetherModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = together::chat_model("your-api-key", TogetherModel::Llama3_3_70bInstructTurbo.to_string(), Arc::new(HttpBackend::new()));
```

### Builder 方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

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

使用未列出的模型，直接传入字符串：

```rust,ignore
let model = together::chat_model("your-api-key", "custom-org/custom-model-v1", Arc::new(HttpBackend::new()));
```

## 可用模型

| 枚举变体 | API 模型 ID | 适用场景 |
|---|---|---|
| `Llama3_3_70bInstructTurbo` | `meta-llama/Llama-3.3-70B-Instruct-Turbo` | 通用（推荐） |
| `Llama3_1_8bInstructTurbo` | `meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo` | 快速、低成本 |
| `Llama3_1_405bInstructTurbo` | `meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo` | 最高质量 |
| `DeepSeekR1` | `deepseek-ai/DeepSeek-R1` | 推理任务 |
| `Qwen2_5_72bInstructTurbo` | `Qwen/Qwen2.5-72B-Instruct-Turbo` | 多语言 |
| `Mixtral8x7bInstruct` | `mistralai/Mixtral-8x7B-Instruct-v0.1` | 长上下文 MoE |
| `Custom(String)` | _(任意)_ | 未列出/预览模型 |

## 使用示例

```rust,ignore
use synaptic::openai::compat::together::{self, TogetherModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = together::chat_model("your-api-key", TogetherModel::Llama3_3_70bInstructTurbo.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("你是一个简洁的助手。"),
    Message::human("Rust 以什么著名？"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## 流式输出

```rust,ignore
use futures::StreamExt;

let request = ChatRequest::new(vec![
    Message::human("用 3 句话解释 Rust 的所有权模型。"),
]);

let mut stream = model.stream_chat(request);
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content);
}
println!();
```

## 错误处理

```rust,ignore
use synaptic::core::SynapticError;

match model.chat(request).await {
    Ok(response) => println!("{}", response.message.content()),
    Err(SynapticError::RateLimit(msg)) => eprintln!("触发限流：{}", msg),
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
