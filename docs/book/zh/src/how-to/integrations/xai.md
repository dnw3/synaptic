# xAI Grok

[xAI](https://x.ai/) 开发了 Grok 系列大语言模型，以实时推理能力和 X（Twitter）数据集成著称。Grok API 与 OpenAI API 兼容。

xAI 作为 `synaptic-models` 内的兼容子模块提供，无需单独的 crate。

## 安装

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
```

在 [x.ai](https://x.ai/) 注册以获取 API 密钥。

## 配置

```rust,ignore
use synaptic::openai::compat::xai::{self, XaiModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = xai::chat_model("xai-your-api-key", XaiModel::Grok2Latest.to_string(), Arc::new(HttpBackend::new()));
```

### Builder 方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

```rust,ignore
use synaptic::openai::compat::xai::{self, XaiModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = xai::config("xai-your-api-key", XaiModel::Grok2Latest.to_string())
    .with_temperature(0.7)
    .with_max_tokens(8192);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

## 可用模型

| 枚举变体 | API 模型 ID | 适用场景 |
|---|---|---|
| `Grok2Latest` | `grok-2-latest` | 通用（推荐） |
| `Grok2Mini` | `grok-2-mini` | 快速、低成本 |
| `GrokBeta` | `grok-beta` | 兼容旧版 |
| `Custom(String)` | _(任意)_ | 预览模型 |

## 使用示例

```rust,ignore
use synaptic::openai::compat::xai::{self, XaiModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = xai::chat_model("xai-your-api-key", XaiModel::Grok2Latest.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("你是 Grok，一个有趣且有用的 AI。"),
    Message::human("今天 AI 领域有什么新进展？"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## 流式输出

```rust,ignore
use futures::StreamExt;

let mut stream = model.stream_chat(ChatRequest::new(vec![
    Message::human("简述今日 AI 趋势。"),
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
