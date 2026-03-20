# Architecture

Synaptic is organized as a workspace of focused Rust crates. In v0.4, 47 fine-grained crates were consolidated into 18 cohesive units. Each crate owns a clear concern, and they compose together through shared traits defined in a single core crate. This page explains the layered design, the principles behind it, and how the crates depend on each other.

## Design Principles

**Async-first.** Every trait in Synaptic is async via `#[async_trait]`, and the runtime is tokio. This is not an afterthought bolted onto a synchronous API -- async is the foundation. LLM calls, tool execution, memory access, and embedding queries are all naturally asynchronous operations, and Synaptic models them as such from the start.

**Feature-gated consolidation.** Each consolidated crate uses feature flags to control which backends and providers are compiled. For example, `synaptic-models` has `openai`, `anthropic`, `gemini`, `ollama`, `bedrock`, and `cohere` features. You only compile the providers you actually use. This keeps compile times manageable while reducing the number of crates to manage.

**Shared traits in core.** The `synaptic-core` crate defines every trait and type that crosses crate boundaries: `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler`, `Message`, `ChatRequest`, `ChatResponse`, `ToolCall`, `SynapticError`, `RunnableConfig`, and more. Implementation crates depend on core, never on each other (unless composition requires it).

**Concurrency-safe by default.** Shared registries use `Arc<RwLock<_>>` (standard library `RwLock` for low-contention read-heavy data like tool registries). Mutable state that requires async access -- callbacks, memory stores, checkpointers -- uses `Arc<tokio::sync::Mutex<_>>` or `Arc<tokio::sync::RwLock<_>>`. All core traits require `Send + Sync`.

**Session isolation.** Memory, agent runs, and graph checkpoints are keyed by a session or thread identifier. Two concurrent conversations never interfere with each other, even when they share the same model and tool instances.

**Event-driven observability.** The `RunEvent` enum captures every significant lifecycle event (run started, LLM called, tool called, run finished, run failed). Callback handlers receive these events asynchronously, enabling logging, tracing, recording, and custom side effects without modifying application code.

## The Four Layers

Synaptic's crates fall into four layers, each building on the ones below it.

### Layer 1: Core

`synaptic-core` is the foundation. It defines:

- **Traits**: `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler`
- **Message types**: The `Message` enum (System, Human, AI, Tool, Chat, Remove), `AIMessageChunk` for streaming, `ToolCall`, `ToolDefinition`, `ToolChoice`
- **Request/response**: `ChatRequest`, `ChatResponse`, `TokenUsage`
- **Streaming**: The `ChatStream` type alias (`Pin<Box<dyn Stream<Item = Result<AIMessageChunk, SynapticError>> + Send>>`)
- **Configuration**: `RunnableConfig` (tags, metadata, concurrency limits, run IDs)
- **Events**: `RunEvent` enum with six lifecycle variants
- **Errors**: `SynapticError` enum with 19 variants spanning all subsystems

Every other crate in the workspace depends on `synaptic-core`. Nothing depends on `synaptic-core` except through this single shared foundation.

### Layer 2: Implementation Crates

Each crate implements one core concern:

| Crate | Purpose |
|-------|---------|
| `synaptic-models` | All LLM providers (`OpenAiChatModel`, `AnthropicChatModel`, `GeminiChatModel`, `OllamaChatModel`, `BedrockChatModel`, `CohereReranker`) + `ProviderBackend` abstraction, test doubles (`ScriptedChatModel`), wrappers (`RetryChatModel`, `RateLimitedChatModel`, `StructuredOutputChatModel<T>`, `BoundToolsChatModel`). Enable providers via feature flags: `openai`, `anthropic`, `gemini`, `ollama`, `bedrock`, `cohere` |
| `synaptic-tools` | `ToolRegistry`, `SerialToolExecutor`, `ParallelToolExecutor`, `HandleErrorTool`, `ReturnDirectTool` + built-in tools: `PdfLoader` (feature `pdf`), Tavily search (feature `tavily`), SQL toolkit (feature `sqltoolkit`) |
| `synaptic-memory` | `ChatMessageHistory` and strategy types: Buffer, Window, Summary, TokenBuffer, SummaryBuffer, `RunnableWithMessageHistory` |
| `synaptic-integrations` | LCEL composition (`Runnable` trait, `BoxRunnable`, pipe operator, `RunnableSequence`, `RunnableParallel`, `RunnableBranch`, `RunnableWithFallbacks`, `RunnableAssign`, `RunnablePick`, `RunnableEach`, `RunnableRetry`, `RunnableGenerator`), prompt templates (`PromptTemplate`, `ChatPromptTemplate`, `FewShotChatMessagePromptTemplate`, `ExampleSelector`), output parsers (string, JSON, structured, list, enum, boolean, XML, markdown list, numbered list, retry, fixing), callbacks (`RecordingCallback`, `TracingCallback`, `CompositeCallback`), LLM caching (`InMemoryCache`, `SemanticCache`, `CachedChatModel`), session management, condenser strategies |
| `synaptic-eval` | Evaluators (ExactMatch, JsonValidity, RegexMatch, EmbeddingDistance, LLMJudge), `Dataset`, batch evaluation pipeline |

### Layer 3: Composition and Retrieval

These crates combine the implementation crates into higher-level abstractions:

| Crate | Purpose |
|-------|---------|
| `synaptic-graph` | LangGraph-style state machines: `StateGraph` builder, `CompiledGraph`, `Node` trait, `ToolNode`, `create_react_agent()`, `StoreCheckpointer`, streaming, visualization |
| `synaptic-rag` | Full RAG pipeline: document loaders (`TextLoader`, `JsonLoader`, `CsvLoader`, `DirectoryLoader`, `FileLoader`, `MarkdownLoader`, `WebLoader`), text splitters (`CharacterTextSplitter`, `RecursiveCharacterTextSplitter`, `MarkdownHeaderTextSplitter`, `HtmlHeaderTextSplitter`, `LanguageTextSplitter`, `TokenTextSplitter`), `Embeddings` trait + `FakeEmbeddings` + `CacheBackedEmbeddings`, vector stores (`VectorStore` trait, `InMemoryVectorStore`, `MultiVectorRetriever`) + backends (Qdrant, pgvector, Pinecone, Chroma, MongoDB, Elasticsearch, Weaviate, Milvus, OpenSearch, LanceDB), retrievers (`Retriever` trait, InMemory, BM25, MultiQuery, Ensemble, ContextualCompression, SelfQuery, ParentDocument). Enable backends via feature flags |
| `synaptic-store` | `Store` trait implementation, `InMemoryStore`, `FileStore` + persistent backends: PostgreSQL (`PgVectorStore`, `PgStore`, `PgCache`, `PgCheckpointer`), Redis (`RedisStore`, `RedisCache`), SQLite, MongoDB. Enable via feature flags: `postgres`, `redis`, `sqlite`, `mongodb` |

### Layer 4: Facade

The `synaptic` crate re-exports everything from all sub-crates under a unified namespace. Application code can use a single dependency:

```toml
[dependencies]
synaptic = "0.4"
```

And then import from organized modules:

```rust
use synaptic::core::{Message, ChatRequest};
use synaptic::openai::OpenAiChatModel;       // requires "openai" feature
use synaptic::anthropic::AnthropicChatModel; // requires "anthropic" feature
use synaptic::graph::{create_react_agent, MessageState};
use synaptic::runnables::{BoxRunnable, Runnable};
```

## Crate Dependency Diagram

```
                       synaptic (facade)
                             |
        +--------------------+--------------------+
        |                    |                    |
   synaptic-graph      synaptic-rag          synaptic-eval
        |                    |                    |
   synaptic-tools        synaptic-core       synaptic-models
        |                    ^                    |
   synaptic-core              |               synaptic-core
                             |
        +--------+-----------+-----------+--------+
        |        |           |           |        |
   synaptic-  synaptic-  synaptic-  synaptic-  synaptic-
   models     memory     integr.    store      rag
        |        |           |           |        |
        +--------+-----------+-----------+--------+
                             |
                        synaptic-core
```

The arrows point downward toward dependencies. Every crate ultimately depends on `synaptic-core`. The composition crates (`synaptic-graph`, `synaptic-rag`) additionally depend on the implementation crates they orchestrate.

## Provider Abstraction

All LLM providers now live in the `synaptic-models` crate, each behind a feature flag (`openai`, `anthropic`, `gemini`, `ollama`, etc.). They all use the `ProviderBackend` trait to separate HTTP concerns from protocol mapping. `HttpBackend` makes real HTTP requests; `FakeBackend` returns scripted responses for testing. This means you can test any code that uses `ChatModel` without network access and without mocking at the HTTP level. You only compile the providers you actually use.

## The Runnable Abstraction

The `Runnable<I, O>` trait in `synaptic-integrations` is the universal composition primitive. Prompt templates, output parsers, chat models, and entire graphs can all be treated as runnables. They compose via the `|` pipe operator into chains that can be invoked, batched, or streamed. See [Runnables & LCEL](./runnables-lcel.md) for details.

## The Graph Abstraction

The `StateGraph` builder in `synaptic-graph` provides a higher-level orchestration model for complex workflows. Where LCEL chains are linear pipelines (with branching), graphs support cycles, conditional routing, checkpointing, human-in-the-loop interrupts, and dynamic control flow via `GraphCommand`. See [Graph](./graph.md) for details.

## See Also

- [Installation](../installation.md) -- feature flags for enabling specific crates
- [Runnables & LCEL](./runnables-lcel.md) -- the composition primitive
- [Graph](./graph.md) -- state-machine orchestration
- [Middleware](./middleware.md) -- cross-cutting agent concerns
- [Key-Value Store](./store.md) -- persistent namespaced storage
