# Perplexity AI

[Perplexity AI](https://www.perplexity.ai/) 通过 Sonar 模型系列提供联网搜索增强的语言模型服务。与传统 LLM 不同，Sonar 模型能够访问实时网络信息并返回引用来源，非常适合事实性查询和研究任务。

Perplexity AI 作为 `synaptic-openai` 内的兼容子模块提供，无需单独的 crate。

## 安装

```toml
[dependencies]
synaptic = { version = "0.3", features = ["openai"] }
```

在 [perplexity.ai](https://www.perplexity.ai/) 注册以获取 API 密钥（以 `pplx-` 开头）。

## 配置

```rust,ignore
use synaptic::openai::compat::perplexity::{self, PerplexityModel};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = perplexity::chat_model("pplx-your-api-key", PerplexityModel::SonarLarge.to_string(), Arc::new(HttpBackend::new()));
```

### Builder 方法

使用 `OpenAiConfig` 的构建器方法进行自定义：

```rust,ignore
use synaptic::openai::compat::perplexity::{self, PerplexityModel};
use synaptic::openai::OpenAiChatModel;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let config = perplexity::config("pplx-your-api-key", PerplexityModel::SonarLarge.to_string())
    .with_temperature(0.2)
    .with_max_tokens(1024);

let model = OpenAiChatModel::new(config, Arc::new(HttpBackend::new()));
```

## 可用模型

| 枚举变体 | API 模型 ID | 适用场景 |
|---|---|---|
| `SonarLarge` | `sonar-large-online` | 通用联网搜索（推荐） |
| `SonarSmall` | `sonar-small-online` | 快速、低成本联网搜索 |
| `SonarHuge` | `sonar-huge-online` | 最高质量联网搜索 |
| `SonarReasoningPro` | `sonar-reasoning-pro` | 带引用的复杂推理 |
| `Custom(String)` | _(任意)_ | 预览模型 |

## 使用示例

```rust,ignore
use synaptic::openai::compat::perplexity::{self, PerplexityModel};
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::models::HttpBackend;
use std::sync::Arc;

let model = perplexity::chat_model("pplx-your-api-key", PerplexityModel::SonarLarge.to_string(), Arc::new(HttpBackend::new()));

let request = ChatRequest::new(vec![
    Message::system("请精确简洁地回答，并引用来源。"),
    Message::human("Rust 在系统编程中的现状如何？"),
]);

let response = model.chat(request).await?;
println!("{}", response.message.content());
```

## 流式输出

```rust,ignore
use futures::StreamExt;

let mut stream = model.stream_chat(ChatRequest::new(vec![
    Message::human("大语言模型研究的最新进展是什么？"),
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
