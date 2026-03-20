# Architecture Overview

Synaptic is organized as a Cargo workspace with 18 crates forming a layered architecture. The v0.4 consolidation merged 47+ fine-grained crates into focused, cohesive units while preserving the same public API surface through the `synaptic` facade.

## Layered Diagram

```text
                          ┌───────────┐
                          │ synaptic  │  (facade: feature-gated re-exports)
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
              │    synaptic-core    │  (traits, types, errors)
              │    synaptic-macros  │  (proc macros)
              └─────────────────────┘
```

## Crate Reference

### Core Layer

| Crate | Purpose |
|---|---|
| `synaptic-core` | Core traits (`ChatModel`, `Tool`, `RuntimeAwareTool`, `Store`, `Embeddings`, `VectorStore`, `Runnable`), types (`Message`, `ChatRequest`, `ChatResponse`, `ToolCall`, `AIMessageChunk`, `ContentBlock`, `Item`), error type (`SynapticError`), stream type (`ChatStream`) |
| `synaptic-macros` | Procedural macros: `#[tool]`, `#[chain]`, `#[entrypoint]`, `#[task]`, `#[traceable]` |

### Model Layer

| Crate | Purpose |
|---|---|
| `synaptic-models` | All chat model providers (OpenAI, Anthropic, Gemini, Ollama, Bedrock, Cohere) + OpenAI-compatible adapters (Groq, DeepSeek, Mistral, Together, Fireworks, xAI, Perplexity). Also: `ProviderBackend` abstraction, `ScriptedChatModel` test double, wrappers (retry, rate limit, structured output, bound tools) |

### RAG Layer

| Crate | Purpose |
|---|---|
| `synaptic-rag` | Full RAG pipeline: prompts, parsers, loaders, splitters, embeddings, vectorstores (Qdrant, Pinecone, Chroma, Elasticsearch, OpenSearch, Milvus, Weaviate, LanceDB), retrieval strategies, evaluation |

### Storage Layer

| Crate | Purpose |
|---|---|
| `synaptic-store` | Data persistence backends: PostgreSQL (`PgVectorStore`, `PgStore`, `PgCache`, `PgCheckpointer`), Redis (`RedisStore`, `RedisCache`), SQLite, MongoDB |

### Agent Layer

| Crate | Purpose |
|---|---|
| `synaptic-graph` | Graph orchestration: `StateGraph`, `CompiledGraph`, `create_react_agent`, `InterceptorChain`, `ToolNode`, `StoreCheckpointer`, multi-mode streaming |
| `synaptic-middleware` | `Interceptor` trait + 12 built-in interceptors (model retry, circuit breaker, model fallback, tool retry, SSRF guard, summarization, human-in-the-loop, tool call limiting, security, context editing) + condenser strategies |
| `synaptic-deep` | Deep Agent harness: `create_deep_agent()`, ACP protocol, 7 built-in tools, backends (State/Store/Filesystem) |
| `synaptic-memory` | Memory strategies: buffer, window, summary, token buffer |

### Infrastructure Layer

| Crate | Purpose |
|---|---|
| `synaptic-events` | `EventBus` + 29 `EventKind` types + 5 dispatch modes + observer/metrics |
| `synaptic-logging` | Structured logging: `LogBuffer`, `LogID`, `MemoryLogLayer` |
| `synaptic-config` | Agent config loading + secrets masking + session + cache + plugin system |

### Integration Layer

| Crate | Purpose |
|---|---|
| `synaptic-integrations` | Third-party services: Tavily, Confluence, Slack, voice, scheduler, Langfuse |
| `synaptic-tools` | Built-in tools: PDF, SQL, E2B, browser, sandbox |
| `synaptic-mcp` | Model Context Protocol client: `MultiServerMcpClient`, Stdio/SSE/HTTP transports |
| `synaptic-lark` | Lark/Feishu bot framework + APIs |

### Facade

**`synaptic`** re-exports all crates with feature gates for convenient single-import usage:

```rust
use synaptic::core::{ChatModel, Message, ChatRequest};
use synaptic::models::OpenAiChatModel;        // requires "openai" feature
use synaptic::graph::{StateGraph, create_react_agent};
use synaptic::rag::{Retriever, RecursiveCharacterTextSplitter};
```

## Design Principles

### Async-first with `#[async_trait]`

Every trait in Synaptic is async. `ChatModel::chat()`, `Tool::call()`, `Store::get()`, and `Runnable::invoke()` are all async functions. You can freely `await` network calls, database queries, and concurrent operations inside any implementation without blocking the runtime.

### `Arc`-based sharing

Synaptic uses `Arc<RwLock<_>>` for registries where many readers need concurrent access, and `Arc<tokio::sync::Mutex<_>>` for stateful components where mutations must be serialized. This allows safe sharing across async tasks and agent sessions.

### Session isolation

Memory stores and agent runs are keyed by `session_id`. Multiple conversations can run concurrently on the same model and tool set without state leaking between sessions.

### Event-driven architecture

The `EventBus` in `synaptic-events` provides 29 event kinds with 5 dispatch modes (sync, async, broadcast, filtered, batched), enabling decoupled observability, metrics, and side effects.

### Typed error handling

`SynapticError` has one variant per subsystem (`Prompt`, `Model`, `Tool`, `Memory`, `Graph`, etc.). This makes it straightforward to match on specific failure modes and provide targeted recovery logic.

### Composition over inheritance

Rather than deep trait hierarchies, Synaptic favors composition. A `CachedChatModel` wraps any `ChatModel`. A `RetryChatModel` wraps any `ChatModel`. Middleware interceptors chain around any agent. You stack behaviors by wrapping, not by extending base classes.
