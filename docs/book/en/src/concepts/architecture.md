# Architecture

Synapse is organized as a workspace of focused Rust crates. Each crate owns exactly one concern, and they compose together through shared traits defined in a single core crate. This page explains the layered design, the principles behind it, and how the crates depend on each other.

## Design Principles

**Async-first.** Every trait in Synapse is async via `#[async_trait]`, and the runtime is tokio. This is not an afterthought bolted onto a synchronous API -- async is the foundation. LLM calls, tool execution, memory access, and embedding queries are all naturally asynchronous operations, and Synapse models them as such from the start.

**One crate, one concern.** The `synapse-models` crate knows how to talk to LLM providers. The `synapse-tools` crate knows how to register and execute tools. The `synapse-memory` crate knows how to store and retrieve conversation history. No crate does two jobs. This keeps compile times manageable, makes it possible to use only what you need, and ensures that changes to one subsystem do not cascade across the codebase.

**Shared traits in core.** The `synapse-core` crate defines every trait and type that crosses crate boundaries: `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler`, `Message`, `ChatRequest`, `ChatResponse`, `ToolCall`, `SynapseError`, `RunnableConfig`, and more. Implementation crates depend on core, never on each other (unless composition requires it).

**Concurrency-safe by default.** Shared registries use `Arc<RwLock<_>>` (standard library `RwLock` for low-contention read-heavy data like tool registries). Mutable state that requires async access -- callbacks, memory stores, checkpointers -- uses `Arc<tokio::sync::Mutex<_>>` or `Arc<tokio::sync::RwLock<_>>`. All core traits require `Send + Sync`.

**Session isolation.** Memory, agent runs, and graph checkpoints are keyed by a session or thread identifier. Two concurrent conversations never interfere with each other, even when they share the same model and tool instances.

**Event-driven observability.** The `RunEvent` enum captures every significant lifecycle event (run started, LLM called, tool called, run finished, run failed). Callback handlers receive these events asynchronously, enabling logging, tracing, recording, and custom side effects without modifying application code.

## The Four Layers

Synapse's crates fall into four layers, each building on the ones below it.

### Layer 1: Core

`synapse-core` is the foundation. It defines:

- **Traits**: `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler`
- **Message types**: The `Message` enum (System, Human, AI, Tool, Chat, Remove), `AIMessageChunk` for streaming, `ToolCall`, `ToolDefinition`, `ToolChoice`
- **Request/response**: `ChatRequest`, `ChatResponse`, `TokenUsage`
- **Streaming**: The `ChatStream` type alias (`Pin<Box<dyn Stream<Item = Result<AIMessageChunk, SynapseError>> + Send>>`)
- **Configuration**: `RunnableConfig` (tags, metadata, concurrency limits, run IDs)
- **Events**: `RunEvent` enum with six lifecycle variants
- **Errors**: `SynapseError` enum with 19 variants spanning all subsystems

Every other crate in the workspace depends on `synapse-core`. Nothing depends on `synapse-core` except through this single shared foundation.

### Layer 2: Implementation Crates

Each crate implements one core concern:

| Crate | Purpose |
|-------|---------|
| `synapse-models` | Provider adapters (OpenAI, Anthropic, Gemini, Ollama), test doubles (`ScriptedChatModel`), wrappers (`RetryChatModel`, `RateLimitedChatModel`, `StructuredOutputChatModel<T>`) |
| `synapse-tools` | `ToolRegistry`, `SerialToolExecutor`, `ParallelToolExecutor`, `HandleErrorTool`, `ReturnDirectTool` |
| `synapse-memory` | `InMemoryStore` and strategy types: Buffer, Window, Summary, TokenBuffer, SummaryBuffer, `RunnableWithMessageHistory`, `FileChatMessageHistory` |
| `synapse-callbacks` | `RecordingCallback`, `TracingCallback`, `CompositeCallback` |
| `synapse-prompts` | `PromptTemplate`, `ChatPromptTemplate`, `FewShotChatMessagePromptTemplate`, `ExampleSelector` |
| `synapse-parsers` | Output parsers: `StrOutputParser`, `JsonOutputParser`, `StructuredOutputParser<T>`, `ListOutputParser`, `EnumOutputParser`, `BooleanOutputParser`, `MarkdownListOutputParser`, `NumberedListOutputParser`, `XmlOutputParser`, `RetryOutputParser`, `FixingOutputParser` |
| `synapse-cache` | `InMemoryCache`, `SemanticCache`, `CachedChatModel` |
| `synapse-eval` | Evaluators (ExactMatch, JsonValidity, RegexMatch, EmbeddingDistance, LLMJudge), `Dataset`, batch evaluation pipeline |

### Layer 3: Composition and Retrieval

These crates combine the implementation crates into higher-level abstractions:

| Crate | Purpose |
|-------|---------|
| `synapse-runnables` | The LCEL system: `Runnable` trait, `BoxRunnable` with pipe operator, `RunnableSequence`, `RunnableParallel`, `RunnableBranch`, `RunnableWithFallbacks`, `RunnableAssign`, `RunnablePick`, `RunnableEach`, `RunnableRetry`, `RunnableGenerator` |
| `synapse-graph` | LangGraph-style state machines: `StateGraph` builder, `CompiledGraph`, `Node` trait, `ToolNode`, `create_react_agent()`, checkpointing, streaming, visualization |
| `synapse-loaders` | Document loaders: `TextLoader`, `JsonLoader`, `CsvLoader`, `DirectoryLoader`, `FileLoader`, `MarkdownLoader`, `WebLoader` |
| `synapse-splitters` | Text splitters: `CharacterTextSplitter`, `RecursiveCharacterTextSplitter`, `MarkdownHeaderTextSplitter`, `HtmlHeaderTextSplitter`, `LanguageTextSplitter`, `TokenTextSplitter` |
| `synapse-embeddings` | `Embeddings` trait, `FakeEmbeddings`, `OpenAiEmbeddings`, `OllamaEmbeddings`, `CachedEmbeddings` |
| `synapse-vectorstores` | `VectorStore` trait, `InMemoryVectorStore`, `MultiVectorRetriever` |
| `synapse-retrieval` | `Retriever` trait and seven implementations: InMemory, BM25, MultiQuery, Ensemble, ContextualCompression, SelfQuery, ParentDocument |

### Layer 4: Facade

The `synapse` crate re-exports everything from all sub-crates under a unified namespace. Application code can use a single dependency:

```toml
[dependencies]
synapse = { path = "crates/synapse" }
```

And then import from organized modules:

```rust
use synapse::core::{Message, ChatRequest};
use synapse::models::OpenAiChatModel;
use synapse::graph::{create_react_agent, MessageState};
use synapse::runnables::{BoxRunnable, Runnable};
```

## Crate Dependency Diagram

```
                        synapse (facade)
                             |
        +--------------------+--------------------+
        |                    |                    |
   synapse-graph      synapse-runnables    synapse-eval
        |                    |                    |
   synapse-tools        synapse-core       synapse-embeddings
        |                    ^                    |
   synapse-core              |               synapse-core
                             |
        +--------+-----------+-----------+--------+
        |        |           |           |        |
   synapse-   synapse-   synapse-   synapse-  synapse-
   models     memory     callbacks  prompts   parsers
        |        |           |           |        |
        +--------+-----------+-----------+--------+
                             |
                        synapse-core

   Retrieval pipeline (all depend on synapse-core):

   synapse-loaders --> synapse-splitters --> synapse-embeddings
                                                   |
                                            synapse-vectorstores
                                                   |
                                            synapse-retrieval
```

The arrows point downward toward dependencies. Every crate ultimately depends on `synapse-core`. The composition crates (`synapse-graph`, `synapse-runnables`) additionally depend on the implementation crates they orchestrate.

## Provider Abstraction

Model adapters in `synapse-models` use the `ProviderBackend` trait to separate HTTP concerns from protocol mapping. `HttpBackend` makes real HTTP requests; `FakeBackend` returns scripted responses for testing. This means you can test any code that uses `ChatModel` without network access and without mocking at the HTTP level.

## The Runnable Abstraction

The `Runnable<I, O>` trait in `synapse-runnables` is the universal composition primitive. Prompt templates, output parsers, chat models, and entire graphs can all be treated as runnables. They compose via the `|` pipe operator into chains that can be invoked, batched, or streamed. See [Runnables & LCEL](./runnables-lcel.md) for details.

## The Graph Abstraction

The `StateGraph` builder in `synapse-graph` provides a higher-level orchestration model for complex workflows. Where LCEL chains are linear pipelines (with branching), graphs support cycles, conditional routing, checkpointing, human-in-the-loop interrupts, and dynamic control flow via `GraphCommand`. See [Graph](./graph.md) for details.
