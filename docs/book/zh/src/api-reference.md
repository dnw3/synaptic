# API 参考

Synapse 的完整 API 文档由 `rustdoc` 生成。你可以在本地构建并浏览：

```bash
cargo doc --workspace --open
```

此命令会为所有 crate 生成文档并在浏览器中打开。

## Crate 一览

下表列出了 Synapse 的所有 crate 及其职责：

| Crate | 说明 |
|---|---|
| [`synapse`](https://docs.rs/synapse) | 统一 facade crate，重新导出所有子 crate |
| [`synapse-core`](https://docs.rs/synapse-core) | 核心 trait 和类型：`ChatModel`、`Message`、`Tool`、`SynapseError`、`RunnableConfig` 等 |
| [`synapse-models`](https://docs.rs/synapse-models) | LLM 提供商适配器（OpenAI、Anthropic、Gemini、Ollama）及装饰器（重试、速率限制、结构化输出） |
| [`synapse-prompts`](https://docs.rs/synapse-prompts) | 提示模板：`PromptTemplate`、`ChatPromptTemplate`、`FewShotChatMessagePromptTemplate` |
| [`synapse-parsers`](https://docs.rs/synapse-parsers) | 输出解析器：`StrOutputParser`、`JsonOutputParser`、`StructuredOutputParser`、`ListOutputParser`、`EnumOutputParser` 等 |
| [`synapse-runnables`](https://docs.rs/synapse-runnables) | LCEL 组合原语：`Runnable` trait、`BoxRunnable`、管道运算符、`RunnableParallel`、`RunnableBranch` 等 |
| [`synapse-tools`](https://docs.rs/synapse-tools) | 工具注册表（`ToolRegistry`）和串行执行器（`SerialToolExecutor`） |
| [`synapse-memory`](https://docs.rs/synapse-memory) | 会话记忆策略：Buffer、Window、Summary、Token Buffer、Summary Buffer |
| [`synapse-graph`](https://docs.rs/synapse-graph) | LangGraph 风格状态机：`StateGraph`、`CompiledGraph`、`ToolNode`、`create_react_agent` |
| [`synapse-callbacks`](https://docs.rs/synapse-callbacks) | 回调处理器：`RecordingCallback`、`TracingCallback`、`CompositeCallback` |
| [`synapse-cache`](https://docs.rs/synapse-cache) | LLM 缓存：`InMemoryCache`（可选 TTL）、`SemanticCache`（嵌入相似度匹配） |
| [`synapse-loaders`](https://docs.rs/synapse-loaders) | 文档加载器：`TextLoader`、`JsonLoader`、`CsvLoader`、`DirectoryLoader` |
| [`synapse-splitters`](https://docs.rs/synapse-splitters) | 文本分割器：`CharacterTextSplitter`、`RecursiveCharacterTextSplitter`、`MarkdownHeaderTextSplitter`、`TokenTextSplitter` |
| [`synapse-embeddings`](https://docs.rs/synapse-embeddings) | 嵌入模型：`OpenAiEmbeddings`、`OllamaEmbeddings`、`FakeEmbeddings` |
| [`synapse-vectorstores`](https://docs.rs/synapse-vectorstores) | 向量存储：`InMemoryVectorStore`（cosine 相似度）、`VectorStoreRetriever` |
| [`synapse-retrieval`](https://docs.rs/synapse-retrieval) | 检索器：`BM25Retriever`、`MultiQueryRetriever`、`EnsembleRetriever`、`SelfQueryRetriever`、`ParentDocumentRetriever` 等 |
| [`synapse-eval`](https://docs.rs/synapse-eval) | 评估器：`ExactMatchEvaluator`、`RegexMatchEvaluator`、`LLMJudgeEvaluator` 等 |

## 常用导入

使用 `synapse` facade crate 时的常用导入路径：

```rust
// 核心类型
use synapse::core::{ChatModel, Message, ChatRequest, ChatResponse, SynapseError};
use synapse::core::{Tool, ToolCall, ToolChoice, ToolDefinition};
use synapse::core::{RunnableConfig, TokenUsage, RunEvent};
use synapse::core::{AIMessageChunk, ChatStream};

// 模型
use synapse::models::{OpenAiChatModel, AnthropicChatModel, GeminiChatModel, OllamaChatModel};
use synapse::models::{ScriptedChatModel, RetryChatModel, RateLimitedChatModel};
use synapse::models::StructuredOutputChatModel;

// Runnables
use synapse::runnables::{Runnable, BoxRunnable, RunnableLambda};
use synapse::runnables::{RunnableParallel, RunnableBranch, RunnablePassthrough};
use synapse::runnables::{RunnableAssign, RunnablePick, RunnableWithFallbacks};

// Prompts
use synapse::prompts::{ChatPromptTemplate, MessageTemplate};
use synapse::prompts::FewShotChatMessagePromptTemplate;

// Parsers
use synapse::parsers::{StrOutputParser, JsonOutputParser, StructuredOutputParser};

// Graph
use synapse::graph::{StateGraph, CompiledGraph, MessageState, ToolNode};
use synapse::graph::{create_react_agent, StreamMode, GraphEvent};
use synapse::graph::MemorySaver;

// Retrieval
use synapse::retrieval::{Retriever, InMemoryRetriever, BM25Retriever};
use synapse::vectorstores::{InMemoryVectorStore, VectorStoreRetriever};
use synapse::embeddings::{OpenAiEmbeddings, FakeEmbeddings};
```

## 构建文档

```bash
# 构建所有 crate 的文档
cargo doc --workspace --open

# 构建单个 crate 的文档
cargo doc -p synapse-core --open

# 包含私有项的文档（开发者参考）
cargo doc --workspace --document-private-items --open
```
