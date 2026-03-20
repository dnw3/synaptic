# 安装

## 前置要求

- **Rust 1.88 或更高版本** -- Synaptic 的最低支持 Rust 版本（MSRV）为 1.88。使用 `rustup update` 更新你的工具链。
- **Cargo** -- Rust 的包管理器，随 Rust 一起安装。

## 添加依赖

### 使用 facade crate（推荐）

`synaptic` facade crate 重新导出所有子 crate。使用 **feature flags** 控制编译哪些模块。

### Feature Flags

Synaptic 提供类似 tokio 的细粒度 feature flags：

```toml
[dependencies]
# 全量引入
synaptic = { version = "0.4", features = ["full"] }

# Agent 开发（tools + graph + memory + middleware + 选择的 Provider）
synaptic = { version = "0.4", features = ["openai", "agent"] }

# RAG 应用（retrieval + loaders + splitters + embeddings + vectorstores）
synaptic = { version = "0.4", features = ["openai", "rag"] }

# Agent + RAG
synaptic = { version = "0.4", features = ["agent", "rag"] }

# 最小化 — 只要 OpenAI 模型调用
synaptic = { version = "0.4", features = ["openai"] }

# 全部提供商
synaptic = { version = "0.4", features = ["models"] }

# 精细控制：单个提供商 + 特定模块
synaptic = { version = "0.4", features = ["anthropic", "graph", "middleware"] }
```

**组合 features：**

| Feature | 说明 |
|---------|------|
| **`default`** | `runnables`、`prompts`、`parsers`、`tools`、`callbacks` |
| **`agent`** | `default` + `openai`、`graph`、`memory`、`middleware`、`store`、`condenser`、`secrets`、`config`、`session` |
| **`rag`** | `default` + `openai`、`embeddings`、`retrieval`、`loaders`、`splitters`、`vectorstores` |
| **`models`** | 全部提供商：`openai` + `anthropic` + `gemini` + `ollama` + `bedrock` + `cohere` |
| **`full`** | 启用全部 features（所有提供商、集成、otel、langfuse、store-filesystem、deep-config） |

**提供商 features**（每个在 `synaptic-models` 中启用一个提供商）：

| Feature | 说明 |
|---------|------|
| `openai` | OpenAI（`OpenAiChatModel`、`OpenAiEmbeddings`） |
| `anthropic` | Anthropic（`AnthropicChatModel`） |
| `gemini` | Google Gemini（`GeminiChatModel`） |
| `ollama` | Ollama（`OllamaChatModel`、`OllamaEmbeddings`） |
| `bedrock` | AWS Bedrock（`BedrockChatModel`） |
| `cohere` | Cohere（`CohereReranker`） |

OpenAI 兼容提供商（Groq、DeepSeek、Mistral、Together、Fireworks、xAI、Perplexity）通过各自的 feature flag 启用：`groq`、`deepseek`、`mistral`、`together`、`fireworks`、`xai`、`perplexity`。

**模块 features：**

| Feature | 说明 |
|---------|------|
| `graph` | 图编排（`StateGraph`、`create_react_agent`、`InterceptorChain`） |
| `middleware` | 拦截器链（工具调用限制、人机协作、摘要、SSRF 防护、熔断器） |
| `memory` | 记忆策略（buffer、window、summary、token buffer） |
| `store` | 持久化后端（postgres、redis、sqlite、mongodb） |
| `mcp` | Model Context Protocol 客户端（Stdio/SSE/HTTP 传输） |
| `macros` | 过程宏（`#[tool]`、`#[chain]`、`#[entrypoint]`、`#[traceable]`） |
| `deep` | Deep Agent 框架（ACP 协议、内置工具、子 Agent、技能） |
| `events` | EventBus，29 种事件类型 + 5 种分发模式 |
| `config` | Agent 配置加载 + 密钥脱敏 + 插件系统 |

**集成 features：**

| Feature | 说明 |
|---------|------|
| `qdrant` | Qdrant 向量存储（通过 `synaptic-rag`） |
| `postgres` | PostgreSQL 存储、缓存、向量存储、检查点（通过 `synaptic-store`） |
| `redis` | Redis 存储 + 缓存（通过 `synaptic-store`） |
| `sqlite` | SQLite 存储（通过 `synaptic-store`） |
| `mongodb` | MongoDB 存储（通过 `synaptic-store`） |
| `pinecone` | Pinecone 向量存储（通过 `synaptic-rag`） |
| `chroma` | Chroma 向量存储（通过 `synaptic-rag`） |
| `elasticsearch` | Elasticsearch 向量存储（通过 `synaptic-rag`） |
| `opensearch` | OpenSearch 向量存储（通过 `synaptic-rag`） |
| `milvus` | Milvus 向量存储（通过 `synaptic-rag`） |
| `lancedb` | LanceDB 向量存储（通过 `synaptic-rag`） |
| `weaviate` | Weaviate 向量存储（通过 `synaptic-rag`） |
| `pdf` | PDF 文档加载器（通过 `synaptic-tools`） |
| `tavily` | Tavily 搜索工具（通过 `synaptic-integrations`） |
| `confluence` | Confluence 集成（通过 `synaptic-integrations`） |
| `slack` | Slack 集成（通过 `synaptic-integrations`） |
| `lark` | 飞书/Lark Bot 框架（通过 `synaptic-lark`） |
| `otel` | OpenTelemetry 追踪 |
| `langfuse` | Langfuse 可观测性 |

`core` 模块（核心 trait 和类型）始终可用，不受 feature 选择影响。

### 常用依赖组合

**基础 LLM 调用（OpenAI）：**

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

**带工具调用的 Agent：**

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai", "agent"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

**RAG 应用：**

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai", "rag"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

然后在代码中使用：

```rust
use synaptic::core::{ChatModel, Message, ChatRequest};
use synaptic::models::OpenAiChatModel;
```

### 按需引入单个 crate

如果你只需要特定功能，也可以单独添加所需的 crate 以缩短编译时间：

```toml
[dependencies]
synaptic-core = "0.4"
synaptic-models = { version = "0.4", features = ["openai"] }
synaptic-graph = "0.4"
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
cargo new my-synaptic-app
cd my-synaptic-app
```

在 `Cargo.toml` 中添加依赖后，运行：

```bash
cargo build
```

如果编译成功，说明安装完成。接下来可以前往[快速开始](quickstart.md)编写你的第一个 Synaptic 程序。
