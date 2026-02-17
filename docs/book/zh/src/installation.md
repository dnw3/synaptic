# 安装

## 前置要求

- **Rust 1.78 或更高版本** -- Synapse 的最低支持 Rust 版本（MSRV）为 1.78。使用 `rustup update` 更新你的工具链。
- **Cargo** -- Rust 的包管理器，随 Rust 一起安装。

## 添加依赖

### 使用 facade crate（推荐）

最简单的方式是添加 `synapse` facade crate，它重新导出了所有子 crate：

```toml
[dependencies]
synapse = "0.1"
tokio = { version = "1.41", features = ["macros", "rt-multi-thread"] }
```

然后在代码中使用：

```rust
use synapse::core::{ChatModel, Message, ChatRequest};
use synapse::models::OpenAiChatModel;
```

### 按需引入单个 crate

如果你只需要特定功能，可以单独添加所需的 crate 以减少编译时间：

```toml
[dependencies]
synapse-core = "0.1"
synapse-models = "0.1"
synapse-runnables = "0.1"
tokio = { version = "1.41", features = ["macros", "rt-multi-thread"] }
```

### 常用依赖组合

以下是几种常见场景的依赖配置：

**基础 LLM 调用：**

```toml
[dependencies]
synapse-core = "0.1"
synapse-models = "0.1"
tokio = { version = "1.41", features = ["macros", "rt-multi-thread"] }
```

**带工具调用的 Agent：**

```toml
[dependencies]
synapse-core = "0.1"
synapse-models = "0.1"
synapse-tools = "0.1"
synapse-graph = "0.1"
tokio = { version = "1.41", features = ["macros", "rt-multi-thread"] }
serde_json = "1.0"
async-trait = "0.1"
```

**RAG 应用：**

```toml
[dependencies]
synapse-core = "0.1"
synapse-models = "0.1"
synapse-embeddings = "0.1"
synapse-vectorstores = "0.1"
synapse-retrieval = "0.1"
synapse-loaders = "0.1"
synapse-splitters = "0.1"
tokio = { version = "1.41", features = ["macros", "rt-multi-thread"] }
```

## 环境变量

根据你使用的 LLM 提供商，需要设置相应的 API 密钥环境变量：

| 提供商 | 环境变量 |
|---|---|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Google Gemini | `GOOGLE_API_KEY` |
| Ollama | 无需密钥（默认连接 `http://localhost:11434`） |

可以通过 `.env` 文件或直接在 shell 中设置：

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

> **注意：** 使用 `ScriptedChatModel`（测试替身）时不需要任何 API 密钥，非常适合本地开发和测试。

## 验证安装

创建一个新项目并验证安装是否成功：

```bash
cargo new my-synapse-app
cd my-synapse-app
```

在 `Cargo.toml` 中添加依赖后，运行：

```bash
cargo build
```

如果编译成功，说明安装完成。接下来可以前往[快速开始](quickstart.md)编写你的第一个 Synapse 程序。
