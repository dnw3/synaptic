# Architecture Overview

Synapse is organized as a Cargo workspace with 17 library crates, 1 facade crate, and several example binaries. The crates form a layered architecture where each layer builds on the one below it.

## Crate Layers

### Core Layer

**`synapse-core`** defines all shared traits and types. Every other crate depends on it.

- Traits: `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler`
- Types: `Message`, `ChatRequest`, `ChatResponse`, `ToolCall`, `ToolDefinition`, `ToolChoice`, `AIMessageChunk`, `TokenUsage`, `RunEvent`, `RunnableConfig`
- Error type: `SynapseError` (19 variants covering all subsystems)
- Stream type: `ChatStream` (`Pin<Box<dyn Stream<Item = Result<AIMessageChunk, SynapseError>> + Send>>`)

### Implementation Crates

Each crate implements one core trait or provides a focused capability:

| Crate | Purpose |
|---|---|
| `synapse-models` | Provider adapters (OpenAI, Anthropic, Gemini, Ollama) + `ScriptedChatModel` test double + wrappers (retry, rate limit, caching, structured output) |
| `synapse-tools` | `ToolRegistry` and `SerialToolExecutor` |
| `synapse-memory` | Memory strategies: buffer, window, summary, token buffer, summary buffer, `RunnableWithMessageHistory` |
| `synapse-callbacks` | `RecordingCallback`, `TracingCallback`, `CompositeCallback` |
| `synapse-prompts` | `PromptTemplate`, `ChatPromptTemplate`, `FewShotChatMessagePromptTemplate` |
| `synapse-parsers` | Output parsers: string, JSON, structured, list, enum, boolean, XML, markdown list, numbered list |
| `synapse-cache` | `InMemoryCache`, `SemanticCache`, `CachedChatModel` |

### Composition Crates

These crates provide higher-level orchestration:

| Crate | Purpose |
|---|---|
| `synapse-runnables` | `Runnable` trait with `invoke()`/`batch()`/`stream()`, `BoxRunnable` with pipe operator, `RunnableLambda`, `RunnableParallel`, `RunnableBranch`, `RunnableAssign`, `RunnablePick`, `RunnableWithFallbacks` |
| `synapse-graph` | LangGraph-style state machines: `StateGraph`, `CompiledGraph`, `ToolNode`, `create_react_agent`, `Checkpointer`, `MemorySaver`, graph streaming |

### Retrieval Pipeline

These crates form the document ingestion and retrieval pipeline:

| Crate | Purpose |
|---|---|
| `synapse-loaders` | `TextLoader`, `JsonLoader`, `CsvLoader`, `DirectoryLoader` |
| `synapse-splitters` | `CharacterTextSplitter`, `RecursiveCharacterTextSplitter`, `MarkdownHeaderTextSplitter`, `TokenTextSplitter` |
| `synapse-embeddings` | `Embeddings` trait, `OpenAiEmbeddings`, `OllamaEmbeddings`, `FakeEmbeddings` |
| `synapse-vectorstores` | `VectorStore` trait, `InMemoryVectorStore`, `VectorStoreRetriever` |
| `synapse-retrieval` | `Retriever` trait, `BM25Retriever`, `MultiQueryRetriever`, `EnsembleRetriever`, `ContextualCompressionRetriever`, `SelfQueryRetriever`, `ParentDocumentRetriever` |

### Evaluation

| Crate | Purpose |
|---|---|
| `synapse-eval` | `Evaluator` trait, `ExactMatchEvaluator`, `RegexMatchEvaluator`, `JsonValidityEvaluator`, `EmbeddingDistanceEvaluator`, `LLMJudgeEvaluator`, `Dataset`, batch evaluation pipeline |

### Facade

**`synapse`** re-exports all sub-crates for convenient single-import usage:

```rust
use synapse::core::{ChatModel, Message, ChatRequest};
use synapse::models::OpenAiChatModel;
use synapse::runnables::{Runnable, RunnableLambda};
use synapse::graph::{StateGraph, create_react_agent};
```

## Dependency Diagram

All crates depend on `synapse-core` for shared traits and types. Higher-level crates depend on the layer below:

```text
                         ┌─────────┐
                         │ synapse │  (facade: re-exports all)
                         └────┬────┘
                              │
       ┌──────────────────────┼──────────────────────┐
       │                      │                      │
  ┌────┴─────┐          ┌────┴─────┐          ┌─────┴────┐
  │  graph   │          │runnables │          │   eval   │
  └────┬─────┘          └────┬─────┘          └─────┬────┘
       │                     │                      │
  ┌────┼────────┬────────────┼──────────┬───────────┤
  │    │        │            │          │           │
┌─┴──┐┌┴───┐┌──┴──┐┌───────┐┌┴──────┐┌─┴────┐┌────┴───┐
│mod-││too- ││mem- ││promp- ││pars- ││cache ││callba- │
│els ││ls  ││ory  ││ts    ││ers   ││     ││cks    │
└─┬──┘└─┬──┘└──┬──┘└───┬───┘└──┬───┘└──┬──┘└───┬───┘
  │      │      │       │       │       │       │
  │  ┌───┼──────┼───────┼───────┼───────┤       │
  │  │   │      │       │       │       │       │
  ┌──┴───┴──────┴───────┴───────┴───────┴───────┴──┐
  │              synapse-core                       │
  │  (ChatModel, Tool, Message, SynapseError, ...) │
  └─────────────────────────────────────────────────┘

  Retrieval pipeline:

  loaders ──► splitters ──► embeddings ──► vectorstores ──► retrieval
                                                              │
                                                         synapse-core
```

## Design Principles

### Async-first with `#[async_trait]`

Every trait in Synapse is async. The `ChatModel::chat()` method, `Tool::call()`, `MemoryStore::load()`, and `Runnable::invoke()` are all async functions. This means you can freely `await` network calls, database queries, and concurrent operations inside any implementation without blocking the runtime.

### `Arc`-based sharing

Synapse uses `Arc<RwLock<_>>` for registries (like `ToolRegistry`) where many readers need concurrent access, and `Arc<tokio::sync::Mutex<_>>` for stateful components (like callbacks and memory stores) where mutations must be serialized. This allows safe sharing across async tasks and agent sessions.

### Session isolation

Memory stores and agent runs are keyed by `session_id`. Multiple conversations can run concurrently on the same model and tool set without state leaking between sessions.

### Event-driven callbacks

The `CallbackHandler` trait receives `RunEvent` values at each lifecycle stage (run started, LLM called, tool called, run finished, run failed). You can compose multiple handlers with `CompositeCallback` for logging, tracing, metrics, and recording simultaneously.

### Typed error handling

`SynapseError` has one variant per subsystem (`Prompt`, `Model`, `Tool`, `Memory`, `Graph`, etc.). This makes it straightforward to match on specific failure modes and provide targeted recovery logic.

### Composition over inheritance

Rather than deep trait hierarchies, Synapse favors composition. A `CachedChatModel` wraps any `ChatModel`. A `RetryChatModel` wraps any `ChatModel`. A `RunnableWithFallbacks` wraps any `Runnable`. You stack behaviors by wrapping, not by extending base classes.
