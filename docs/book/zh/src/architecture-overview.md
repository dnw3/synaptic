# 架构概览

Synaptic 采用 Cargo workspace 组织，包含 18 个 crate，形成分层架构。v0.4 版本将 47+ 个细粒度 crate 合并为聚焦、内聚的单元，同时通过 `synaptic` 门面 crate 保持相同的公共 API 表面。

## 分层架构图

```text
                          ┌───────────┐
                          │ synaptic  │  (门面：feature 门控的重导出)
                          └─────┬─────┘
                                │
  ┌─────────────────────────────┼─────────────────────────────┐
  │              ┌──────────────┼──────────────┐              │
  │              │              │              │              │
  │     ┌────────┴───┐  ┌──────┴──────┐  ┌───┴────────┐     │
  │     │   deep     │  │ middleware  │  │   graph    │     │
  │     └────┬───────┘  └──────┬──────┘  └───┬────────┘     │
  │          │                 │              │              │
  │     ┌────┴─────┬───────────┴──────────────┤              │
  │     │          │                          │              │
  │  ┌──┴───┐  ┌──┴────┐  ┌───────┐  ┌──────┴──┐           │
  │  │models│  │memory │  │config │  │ events  │           │
  │  └──┬───┘  └───────┘  └───────┘  └─────────┘           │
  │     │                                                    │
  │  ┌──┴──────┬──────────┬───────────┬───────────┐          │
  │  │  rag   │  store   │  tools   │integrations│          │
  │  └────────┘  └────────┘  └────────┘└───────────┘          │
  │                                                          │
  │  ┌────────┬──────────┬──────────┐                        │
  │  │  mcp  │ logging  │  lark    │                        │
  │  └────────┘  └────────┘  └────────┘                        │
  │                                                          │
  └──────────────────────┬───────────────────────────────────┘
                         │
              ┌──────────┴──────────┐
              │    synaptic-core    │  (trait、类型、错误)
              │    synaptic-macros  │  (过程宏)
              └─────────────────────┘
```

## Crate 参考

### 核心层

| Crate | 职责 |
|---|---|
| `synaptic-core` | 核心 trait（`ChatModel`、`Tool`、`RuntimeAwareTool`、`Store`、`Embeddings`、`VectorStore`、`Runnable`），类型（`Message`、`ChatRequest`、`ChatResponse`、`ToolCall`、`AIMessageChunk`、`ContentBlock`、`Item`），错误类型（`SynapticError`），流类型（`ChatStream`） |
| `synaptic-macros` | 过程宏：`#[tool]`、`#[chain]`、`#[entrypoint]`、`#[task]`、`#[traceable]` |

### 模型层

| Crate | 职责 |
|---|---|
| `synaptic-models` | 所有 Chat Model 提供商（OpenAI、Anthropic、Gemini、Ollama、Bedrock、Cohere）+ OpenAI 兼容适配器（Groq、DeepSeek、Mistral、Together、Fireworks、xAI、Perplexity）。还包括：`ProviderBackend` 抽象、`ScriptedChatModel` 测试替身、包装器（重试、速率限制、结构化输出、绑定工具） |

### RAG 层

| Crate | 职责 |
|---|---|
| `synaptic-rag` | 完整 RAG 流水线：提示词、解析器、加载器、分割器、嵌入向量、向量存储（Qdrant、Pinecone、Chroma、Elasticsearch、OpenSearch、Milvus、Weaviate、LanceDB）、检索策略、评估 |

### 存储层

| Crate | 职责 |
|---|---|
| `synaptic-store` | 数据持久化后端：PostgreSQL（`PgVectorStore`、`PgStore`、`PgCache`、`PgCheckpointer`）、Redis（`RedisStore`、`RedisCache`）、SQLite、MongoDB |

### Agent 层

| Crate | 职责 |
|---|---|
| `synaptic-graph` | 图编排：`StateGraph`、`CompiledGraph`、`create_react_agent`、`InterceptorChain`、`ToolNode`、`StoreCheckpointer`、多模式流 |
| `synaptic-middleware` | `Interceptor` trait + 12 个内置拦截器（模型重试、熔断器、模型降级、工具重试、SSRF 防护、摘要、人工审批、工具调用限制、安全、上下文编辑）+ 压缩策略 |
| `synaptic-deep` | Deep Agent 运行框架：`create_deep_agent()`、ACP 协议、7 个内置工具、后端（State/Store/Filesystem） |
| `synaptic-memory` | 记忆策略：buffer、window、summary、token buffer |

### 基础设施层

| Crate | 职责 |
|---|---|
| `synaptic-events` | `EventBus` + 29 种 `EventKind` + 5 种分发模式 + 观察者/指标 |
| `synaptic-logging` | 结构化日志：`LogBuffer`、`LogID`、`MemoryLogLayer` |
| `synaptic-config` | Agent 配置加载 + 密钥脱敏 + 会话 + 缓存 + 插件系统 |

### 集成层

| Crate | 职责 |
|---|---|
| `synaptic-integrations` | 第三方服务：Tavily、Confluence、Slack、语音、调度器、Langfuse |
| `synaptic-tools` | 内置工具：PDF、SQL、E2B、浏览器、沙箱 |
| `synaptic-mcp` | Model Context Protocol 客户端：`MultiServerMcpClient`、Stdio/SSE/HTTP 传输层 |
| `synaptic-lark` | 飞书/Lark Bot 框架 + API |

### 门面层

**`synaptic`** 通过 feature 门控重导出所有 crate，提供便捷的单一导入方式：

```rust
use synaptic::core::{ChatModel, Message, ChatRequest};
use synaptic::models::OpenAiChatModel;        // 需要 "openai" feature
use synaptic::graph::{StateGraph, create_react_agent};
use synaptic::rag::{Retriever, RecursiveCharacterTextSplitter};
```

## 设计原则

### 异步优先与 `#[async_trait]`

Synaptic 中的每个 trait 都是异步的。`ChatModel::chat()`、`Tool::call()`、`Store::get()` 和 `Runnable::invoke()` 都是异步函数。你可以在任何实现中自由 `await` 网络调用、数据库查询和并发操作，而不会阻塞运行时。

### 基于 `Arc` 的共享

Synaptic 对注册表使用 `Arc<RwLock<_>>`，允许多个读取者并发访问；对有状态组件使用 `Arc<tokio::sync::Mutex<_>>`，确保修改操作串行化。这允许在异步任务和 agent 会话间安全共享。

### 会话隔离

记忆存储和 agent 运行通过 `session_id` 进行键值隔离。多个对话可以在同一模型和工具集上并发运行，状态不会在会话间泄漏。

### 事件驱动架构

`synaptic-events` 中的 `EventBus` 提供 29 种事件类型和 5 种分发模式（同步、异步、广播、过滤、批量），实现解耦的可观测性、指标收集和副作用处理。

### 类型化错误处理

`SynapticError` 为每个子系统提供一个变体（`Prompt`、`Model`、`Tool`、`Memory`、`Graph` 等）。这使得匹配特定的失败模式并提供针对性的恢复逻辑变得简单直接。

### 组合优于继承

Synaptic 倾向于组合而非深层 trait 层次结构。`CachedChatModel` 包装任意 `ChatModel`。`RetryChatModel` 包装任意 `ChatModel`。中间件拦截器链式包装任意 agent。你通过包装来叠加行为，而非通过扩展基类。
