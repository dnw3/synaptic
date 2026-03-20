# Synaptic

[![CI](https://github.com/dnw3/synaptic/actions/workflows/ci.yml/badge.svg)](https://github.com/dnw3/synaptic/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/synaptic.svg)](https://crates.io/crates/synaptic)
[![docs.rs](https://docs.rs/synaptic/badge.svg)](https://docs.rs/synaptic)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange.svg)](https://blog.rust-lang.org/2025/05/15/Rust-1.88.0.html)

A Rust agent framework with LangChain-compatible architecture and Rust-native async interfaces.

## Features

- **LLM Providers** — OpenAI, Anthropic, Gemini, Ollama, AWS Bedrock, and 10 OpenAI-compatible APIs via `compat::` submodules (Groq, DeepSeek, Mistral, Fireworks, Together, xAI, Perplexity, HuggingFace, Cohere, OpenRouter)
- **LCEL Composition** — `Runnable` trait with pipe operator (`|`), streaming, bind, parallel, branch, assign/pick, fallbacks
- **Graph Orchestration** — LangGraph-style `StateGraph` with conditional edges, persistent checkpointing (Redis + PostgreSQL), human-in-the-loop, streaming
- **ReAct Agent** — `create_react_agent(model, tools)` with automatic tool dispatch
- **Tool System** — `Tool` trait, `ToolRegistry`, `SerialToolExecutor`, `ParallelToolExecutor`, built-in tools (Tavily, DuckDuckGo, Wikipedia, SQL Toolkit)
- **Memory** — Buffer, Window, Summary, SummaryBuffer, TokenBuffer strategies with `RunnableWithMessageHistory`
- **Prompt Templates** — Chat templates, few-shot prompting, placeholder interpolation
- **Output Parsers** — String, JSON, Structured\<T\>, List, Boolean, Enum, XML — all composable as `Runnable`
- **RAG Pipeline** — Document loaders (Text, JSON, CSV, Markdown, Directory, Web, PDF), text splitters, embeddings (OpenAI, Ollama, Cohere, HuggingFace), vector stores (InMemory, Qdrant, pgvector, Pinecone, Chroma, MongoDB, Elasticsearch, Weaviate), 7 retriever types
- **Caching** — In-memory (TTL), semantic (embedding similarity), Redis, SQLite, `CachedChatModel` wrapper
- **Evaluation** — ExactMatch, JsonValidity, Regex, EmbeddingDistance, LLMJudge evaluators
- **Structured Output** — `StructuredOutputChatModel<T>` with JSON schema enforcement
- **Observability** — `TracingCallback` (structured spans), `CompositeCallback`, `StdOutCallback`
- **Events** — `EventSubscriber` trait for agent lifecycle events (agent start/stop, model calls, tool calls)
- **MCP** — `MultiServerMcpClient` with Stdio/SSE/HTTP transport adapters
- **Macros** — `#[tool]`, `#[chain]`, `#[entrypoint]`, `#[task]`, `#[traceable]` proc-macros
- **Deep Agent** — Filesystem-aware deep research agent harness (`create_deep_agent`)
- **Interceptors** — `Interceptor` trait with 4 hooks: `before_model`, `after_model`, `wrap_model_call`, `wrap_tool_call`

## Installation

```toml
[dependencies]
synaptic = { version = "0.4", features = ["agent"] }
```

### Feature flags

| Feature | Contents |
|---------|----------|
| `default` | runnables + prompts + parsers + tools |
| `openai` | OpenAI ChatModel + Embeddings + 10 OpenAI-compatible providers via `compat::` |
| `anthropic` | Anthropic Claude ChatModel |
| `gemini` | Google Gemini ChatModel |
| `ollama` | Ollama ChatModel + Embeddings |
| `bedrock` | AWS Bedrock ChatModel |
| `cohere` | Cohere reranker + embeddings |
| `models` | All chat model provider crates (openai + anthropic + gemini + ollama + bedrock + cohere) |
| `qdrant` | Qdrant vector store |
| `postgres` | PostgreSQL store, cache, vector store, graph checkpointer |
| `redis` | Redis store + LLM cache + graph checkpointer |
| `sqlite` | SQLite store |
| `mongodb` | MongoDB store |
| `pinecone` | Pinecone vector store |
| `chroma` | Chroma vector store |
| `weaviate` | Weaviate vector store |
| `elasticsearch` | Elasticsearch vector store |
| `opensearch` | OpenSearch vector store |
| `milvus` | Milvus vector store |
| `lancedb` | LanceDB vector store |
| `huggingface` | HuggingFace Inference API embeddings |
| `voyage` | Voyage AI embeddings |
| `nomic` | Nomic embeddings |
| `jina` | Jina embeddings |
| `flashrank` | FlashRank reranker |
| `tavily` | Tavily search tool |
| `sqltoolkit` | SQL database toolkit (ListTables, DescribeTable, ExecuteQuery) |
| `pdf` | PDF document loader |
| `e2b` | E2B code sandbox |
| `browser` | Browser automation tool |
| `sandbox` | Docker sandbox tool |
| `graph` | LangGraph-style StateGraph |
| `memory` | Conversation memory strategies |
| `middleware` | `Interceptor` trait + `InterceptorChain` |
| `events` | `EventSubscriber` for agent lifecycle events |
| `store` | Key-value store (`InMemoryStore`, `FileStore`) |
| `config` | Agent configuration loading |
| `logging` | Structured logging utilities |
| `retrieval` | Retriever types (BM25, Ensemble, etc.) |
| `cache` | LLM response caching |
| `eval` | Evaluators |
| `mcp` | MCP server client |
| `macros` | Proc-macros |
| `deep` | Deep Agent harness |
| `condenser` | Context condensation (summarization) middleware |
| `secrets` | Secrets masking middleware |
| `session` | Session persistence (graph + memory + store + config) |
| `plugin` | Plugin system (events + config) |
| `otel` | OpenTelemetry integration |
| `langfuse` | Langfuse integration |
| `viking` | Viking memory provider |
| `voice` | Voice integration |
| `scheduler` | Task scheduler |
| `confluence` | Confluence integration |
| `slack` | Slack integration |
| `lark` | Lark (Feishu) integration |
| `lark-bot` | Lark bot framework |
| `agent` | default + graph + memory + middleware + events + plugin + store + condenser + secrets + config + session + logging |
| `agent-openai` | agent + openai |
| `agent-anthropic` | agent + anthropic |
| `agent-ollama` | agent + ollama |
| `rag` | default + embeddings + retrieval + loaders + splitters + vectorstores |
| `rag-openai` | rag + openai |
| `rag-anthropic` | rag + anthropic |
| `rag-ollama` | rag + ollama |
| `full` | Everything |

## Quick Start

```rust
use synaptic::core::{ChatModel, Message, ChatRequest, ToolChoice};
use synaptic::runnables::{Runnable, RunnableLambda};
use synaptic::graph::{create_react_agent, MessageState};

// LCEL pipe composition
let chain = step1.boxed() | step2.boxed() | step3.boxed();
let result = chain.invoke(input, &config).await?;

// ReAct agent
let graph = create_react_agent(model, tools)?;
let state = MessageState { messages: vec![Message::human("Hello")] };
let result = graph.invoke(state).await?;
```

## Workspace Layout

17 crates in `crates/`, examples in `examples/`:

| Crate | Purpose |
|-------|---------|
| `synaptic` | Facade — re-exports all crates with feature gates |
| `synaptic-core` | Core traits, types, errors (`Message`, `ChatModel`, `Tool`, `Store`, `Runnable`, `Embeddings`) |
| `synaptic-models` | Chat model providers: OpenAI, Anthropic, Gemini, Ollama, Bedrock, Cohere + OpenAI-compatible |
| `synaptic-rag` | RAG pipeline: prompts, parsers, loaders, splitters, embeddings, vectorstores, retrieval, eval |
| `synaptic-store` | Storage backends: PostgreSQL, Redis, SQLite, MongoDB |
| `synaptic-graph` | Graph orchestration: `StateGraph`, `CompiledGraph`, `InterceptorChain` |
| `synaptic-middleware` | `Interceptor` trait + 12 built-in interceptors + condenser strategies |
| `synaptic-deep` | Deep Agent harness: `create_deep_agent`, ACP protocol, 7 built-in tools |
| `synaptic-events` | `EventBus` + 29 event kinds + 5 dispatch modes + observer metrics |
| `synaptic-logging` | Structured logging: `LogBuffer`, LogID generation, `MemoryLogLayer` |
| `synaptic-config` | Agent config, secrets masking, session persistence, cache, plugin system |
| `synaptic-memory` | Memory strategies: buffer, window, summary, token buffer |
| `synaptic-tools` | Built-in tools: PDF, SQL, E2B, browser, sandbox |
| `synaptic-integrations` | Third-party services: Tavily, Confluence, Slack, voice, scheduler, Langfuse |
| `synaptic-mcp` | Model Context Protocol client (stdio/SSE/HTTP) |
| `synaptic-lark` | Lark/Feishu bot framework + 15 API modules |
| `synaptic-macros` | Proc macros: `#[tool]`, `#[chain]`, `#[entrypoint]`, Interceptor macros |

### Chat Model Providers

All providers live in `synaptic-models`. Enable via feature flags on the `synaptic` facade:

| Feature | Provider |
|---------|----------|
| `openai` | OpenAI (GPT-4o, o1, o3) + compatible: Groq, DeepSeek, Mistral, Together, Fireworks, xAI, Perplexity |
| `anthropic` | Anthropic (Claude 4.6, Haiku) |
| `gemini` | Google Gemini |
| `ollama` | Ollama (local models) |
| `bedrock` | AWS Bedrock |
| `cohere` | Cohere |

### Embeddings

Embedding providers live in `synaptic-rag`. Enable via feature flags:

| Feature | Provider |
|---------|----------|
| `openai` | OpenAI `text-embedding-3-small/large` |
| `ollama` | Ollama local embedding models |
| `cohere` | Cohere `embed-english-v3.0`, `embed-multilingual-v3.0` |
| `huggingface` | HuggingFace Inference API (BAAI/bge, sentence-transformers...) |
| `voyage` | Voyage AI embeddings |
| `nomic` | Nomic embeddings |
| `jina` | Jina embeddings |

### Vector Stores

Vector store backends live in `synaptic-rag`. Enable via feature flags:

| Feature | Backend |
|---------|---------|
| (default) | In-memory (cosine similarity) |
| `qdrant` | Qdrant |
| `postgres` | PostgreSQL (pgvector) |
| `pinecone` | Pinecone |
| `chroma` | Chroma |
| `mongodb` | MongoDB Atlas Vector Search |
| `elasticsearch` | Elasticsearch |
| `opensearch` | OpenSearch |
| `milvus` | Milvus |
| `lancedb` | LanceDB |

### Store, Cache & Graph Persistence

Storage backends live in `synaptic-store`. Enable via feature flags:

| Feature | Backend |
|---------|---------|
| `postgres` | PostgreSQL (Store + Cache + VectorStore + Graph Checkpointer) |
| `redis` | Redis Store + LLM Cache + Graph Checkpointer |
| `sqlite` | SQLite Store + LLM Cache |
| `mongodb` | MongoDB Store |

### Tools & Integrations

Built-in tools live in `synaptic-tools`; third-party integrations in `synaptic-integrations`. Enable via feature flags:

| Feature | Description |
|---------|-------------|
| `tavily` | Tavily AI search (in `synaptic-integrations`) |
| `sqltoolkit` | ListTables, DescribeTable, ExecuteQuery (in `synaptic-tools`) |
| `pdf` | PDF document loader (in `synaptic-tools`) |
| `e2b` | E2B code sandbox (in `synaptic-tools`) |
| `browser` | Browser automation tool (in `synaptic-tools`) |
| `sandbox` | Docker sandbox tool (in `synaptic-tools`) |
| `confluence` | Confluence integration (in `synaptic-integrations`) |
| `slack` | Slack integration (in `synaptic-integrations`) |
| `langfuse` | Langfuse observability (in `synaptic-integrations`) |

## Examples

```bash
cargo run -p tool_calling_basic   # Tool registry and execution
cargo run -p memory_chat          # Session-based conversation memory
cargo run -p react_basic          # ReAct agent with tool calling
cargo run -p graph_visualization  # Graph state machine visualization
cargo run -p lcel_chain           # LCEL pipe composition and parallel
cargo run -p prompt_parser_chain  # Prompt template -> model -> parser
cargo run -p streaming            # Streaming chat and runnables
cargo run -p rag_pipeline         # RAG: load -> split -> embed -> retrieve
cargo run -p memory_strategy      # Memory strategies comparison
cargo run -p structured_output    # Structured output with JSON schema
cargo run -p callbacks_tracing    # Callbacks and tracing
cargo run -p evaluation           # Evaluator pipeline
cargo run -p caching              # LLM response caching
cargo run -p macros_showcase      # Proc-macro usage
```

All examples use `ScriptedChatModel` and `FakeEmbeddings` — no API keys required.

## Documentation

- **Book**: [dnw3.github.io/synaptic](https://dnw3.github.io/synaptic) — tutorials, how-to guides, concepts, integration reference
- **API Reference**: [docs.rs/synaptic](https://docs.rs/synaptic) — full Rustdoc API documentation

## Design Principles

- Core abstractions first, feature crates expanded incrementally
- LangChain concept compatibility with Rust-idiomatic APIs
- All traits are async via `#[async_trait]`, runtime is tokio
- Type-erased composition via `BoxRunnable` with `|` pipe operator
- `Arc<RwLock<_>>` for shared registries, session-keyed memory isolation
- MSRV: 1.88

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, or the [full guide](https://dnw3.github.io/synaptic/contributing.html).

## License

MIT — see [LICENSE](LICENSE) for details.
