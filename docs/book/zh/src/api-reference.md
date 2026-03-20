# API 参考

Synaptic 的完整 API 文档由 `rustdoc` 生成。你可以在本地构建并浏览：

```bash
cargo doc --workspace --open
```

此命令会为所有 crate 生成文档并在浏览器中打开。

## Crate 一览

v0.4 将原来的 47 个 crate 整合为 18 个。下表列出了 Synaptic 的所有 crate 及其职责：

| Crate | 说明 |
|---|---|
| [`synaptic`](https://docs.rs/synaptic) | 统一 facade crate，重新导出所有子 crate |
| [`synaptic-core`](https://docs.rs/synaptic-core) | 核心 trait 和类型：`ChatModel`、`Message`、`Tool`、`SynapticError`、`RunnableConfig` 等 |
| [`synaptic-models`](https://docs.rs/synaptic-models) | 所有 LLM 提供商（`OpenAiChatModel`、`AnthropicChatModel`、`GeminiChatModel`、`OllamaChatModel` 等）+ `ProviderBackend` 抽象、`ScriptedChatModel` 测试替身、ChatModel 包装器。通过 feature flag 启用：`openai`、`anthropic`、`gemini`、`ollama`、`bedrock`、`cohere` |
| [`synaptic-integrations`](https://docs.rs/synaptic-integrations) | LCEL 组合原语（`Runnable` trait、`BoxRunnable`、管道运算符）、提示模板、输出解析器、回调处理器、会话管理、上下文压缩策略、密钥脱敏 |
| [`synaptic-tools`](https://docs.rs/synaptic-tools) | 工具注册表（`ToolRegistry`）、串行执行器（`SerialToolExecutor`）+ 内置工具：PDF 加载器、Tavily 搜索、SQL 工具包。通过 feature flag 启用：`pdf`、`tavily`、`sqltoolkit` |
| [`synaptic-memory`](https://docs.rs/synaptic-memory) | 会话记忆策略：Buffer、Window、Summary、Token Buffer、Summary Buffer |
| [`synaptic-graph`](https://docs.rs/synaptic-graph) | LangGraph 风格状态机：`StateGraph`、`CompiledGraph`、`ToolNode`、`create_react_agent` |
| [`synaptic-store`](https://docs.rs/synaptic-store) | 键值存储（`InMemoryStore`、`FileStore`）+ 持久化后端：PostgreSQL（`PgStore`、`PgCache`、`PgCheckpointer`）、Redis（`RedisStore`、`RedisCache`、`RedisCheckpointer`）、SQLite（`SqliteCheckpointer`）、MongoDB（`MongoCheckpointer`）。通过 feature flag 启用：`postgres`、`redis`、`sqlite`、`mongodb` |
| [`synaptic-rag`](https://docs.rs/synaptic-rag) | 完整 RAG 管道：文档加载器、文本分割器、嵌入模型、向量存储、检索器 + 向量数据库后端：Qdrant、Pinecone、Chroma、Elasticsearch、pgvector 等。通过 feature flag 启用 |
| [`synaptic-eval`](https://docs.rs/synaptic-eval) | 评估器：`ExactMatchEvaluator`、`RegexMatchEvaluator`、`LLMJudgeEvaluator` 等 |
| [`synaptic-middleware`](https://docs.rs/synaptic-middleware) | `Interceptor` trait、`InterceptorChain`、内置中间件 |
| [`synaptic-mcp`](https://docs.rs/synaptic-mcp) | Model Context Protocol 适配器 |
| [`synaptic-macros`](https://docs.rs/synaptic-macros) | 过程宏：`#[tool]`、`#[chain]`、`#[entrypoint]`、`#[traceable]` |
| [`synaptic-deep`](https://docs.rs/synaptic-deep) | Deep Agent 运行框架 |
| [`synaptic-lark`](https://docs.rs/synaptic-lark) | 飞书/Lark 集成（文档加载器、机器人框架、多维表格检查点） |

## 常用导入

使用 `synaptic` facade crate 时的常用导入路径：

```rust
// 核心类型
use synaptic::core::{ChatModel, Message, ChatRequest, ChatResponse, SynapticError};
use synaptic::core::{Tool, ToolCall, ToolChoice, ToolDefinition};
use synaptic::core::{RunnableConfig, TokenUsage, RunEvent};
use synaptic::core::{AIMessageChunk, ChatStream};

// 提供商模型（均在 synaptic-models 中，通过 feature flag 启用）
use synaptic::openai::OpenAiChatModel;
use synaptic::anthropic::AnthropicChatModel;
use synaptic::gemini::GeminiChatModel;
use synaptic::ollama::OllamaChatModel;

// 模型工具
use synaptic::models::{ScriptedChatModel, RetryChatModel, RateLimitedChatModel};
use synaptic::models::StructuredOutputChatModel;

// Runnables（在 synaptic-integrations 中）
use synaptic::runnables::{Runnable, BoxRunnable, RunnableLambda};
use synaptic::runnables::{RunnableParallel, RunnableBranch, RunnablePassthrough};
use synaptic::runnables::{RunnableAssign, RunnablePick, RunnableWithFallbacks};

// Prompts（在 synaptic-integrations 中）
use synaptic::prompts::{ChatPromptTemplate, MessageTemplate};
use synaptic::prompts::FewShotChatMessagePromptTemplate;

// Parsers（在 synaptic-integrations 中）
use synaptic::parsers::{StrOutputParser, JsonOutputParser, StructuredOutputParser};

// Graph
use synaptic::graph::{StateGraph, CompiledGraph, MessageState, ToolNode};
use synaptic::graph::{create_react_agent, StreamMode, GraphEvent};
use synaptic::graph::StoreCheckpointer;

// Retrieval（在 synaptic-rag 中）
use synaptic::retrieval::{Retriever, InMemoryRetriever, BM25Retriever};
use synaptic::vectorstores::{InMemoryVectorStore, VectorStoreRetriever};
use synaptic::embeddings::FakeEmbeddings;
use synaptic::openai::OpenAiEmbeddings;
```

## 构建文档

```bash
# 构建所有 crate 的文档
cargo doc --workspace --open

# 构建单个 crate 的文档
cargo doc -p synaptic-core --open

# 包含私有项的文档（开发者参考）
cargo doc --workspace --open
```
