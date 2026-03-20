# API Reference

Synaptic is organized as a workspace of 18 focused crates (consolidated from 47 in v0.3). Each crate has its own API documentation generated from doc comments in the source code.

## Crate Reference

| Crate | Description | Docs |
|-------|-------------|------|
| `synaptic-core` | Shared traits and types (`ChatModel`, `Tool`, `Message`, `SynapticError`, etc.) | [docs.rs](https://docs.rs/synaptic-core) |
| `synaptic-models` | All LLM providers (`OpenAiChatModel`, `AnthropicChatModel`, `GeminiChatModel`, `OllamaChatModel`, etc.) + `ProviderBackend` abstraction, `ScriptedChatModel` test double, wrappers (retry, rate limit, structured output, bound tools). Enable providers via feature flags: `openai`, `anthropic`, `gemini`, `ollama`, `bedrock`, `cohere` | [docs.rs](https://docs.rs/synaptic-models) |
| `synaptic-integrations` | LCEL composition (`Runnable` trait, `BoxRunnable`, pipe operator, parallel, branch, fallbacks, assign, pick), prompt templates, output parsers, callback handlers, session management, condenser strategies, secrets masking | [docs.rs](https://docs.rs/synaptic-integrations) |
| `synaptic-tools` | Tool system (`ToolRegistry`, `SerialToolExecutor`, `ParallelToolExecutor`) + built-in tools: PDF loader (`PdfLoader`), Tavily search, SQL toolkit. Enable via feature flags: `pdf`, `tavily`, `sqltoolkit` | [docs.rs](https://docs.rs/synaptic-tools) |
| `synaptic-memory` | Memory strategies (buffer, window, summary, token buffer, summary buffer, `RunnableWithMessageHistory`) | [docs.rs](https://docs.rs/synaptic-memory) |
| `synaptic-graph` | Graph orchestration (`StateGraph`, `CompiledGraph`, `ToolNode`, `create_react_agent`, checkpointing, streaming) | [docs.rs](https://docs.rs/synaptic-graph) |
| `synaptic-store` | Key-value store (`InMemoryStore`, `FileStore`) + persistent backends: PostgreSQL (`PgStore`, `PgCache`, `PgCheckpointer`), Redis (`RedisStore`, `RedisCache`, `RedisCheckpointer`), SQLite (`SqliteCache`, `SqliteCheckpointer`), MongoDB (`MongoCheckpointer`). Enable via feature flags: `postgres`, `redis`, `sqlite`, `mongodb` | [docs.rs](https://docs.rs/synaptic-store) |
| `synaptic-rag` | Full RAG pipeline: document loaders, text splitters, embeddings, vector stores (`InMemoryVectorStore`, `VectorStoreRetriever`, `MultiVectorRetriever`), retrievers (BM25, multi-query, ensemble, contextual compression, self-query, parent document) + vector store backends: Qdrant, Pinecone, Chroma, Elasticsearch, Weaviate, Milvus, OpenSearch, LanceDB, pgvector. Enable via feature flags: `qdrant`, `pinecone`, `chroma`, `elasticsearch`, `weaviate`, `milvus`, `opensearch`, `lancedb`, `pgvector` | [docs.rs](https://docs.rs/synaptic-rag) |
| `synaptic-eval` | Evaluation framework (exact match, regex, JSON validity, embedding distance, LLM judge evaluators; `Dataset` and `evaluate()`) | [docs.rs](https://docs.rs/synaptic-eval) |
| `synaptic-middleware` | `Interceptor` trait, `InterceptorChain`, built-in middleware (model retry, circuit breaker, model fallback, tool retry, SSRF guard, summarization, human-in-the-loop approval, tool call limiting, security) | [docs.rs](https://docs.rs/synaptic-middleware) |
| `synaptic-mcp` | Model Context Protocol adapters (`MultiServerMcpClient`, Stdio/SSE/HTTP transports) | [docs.rs](https://docs.rs/synaptic-mcp) |
| `synaptic-macros` | Procedural macros (`#[tool]`, `#[chain]`, `#[entrypoint]`, `#[task]`, `#[traceable]`) | [docs.rs](https://docs.rs/synaptic-macros) |
| `synaptic-deep` | Deep Agent harness (`Backend` trait, filesystem tools, sub-agents, skills, `create_deep_agent()`) | [docs.rs](https://docs.rs/synaptic-deep) |
| `synaptic-lark` | Feishu/Lark integration (document loaders, bot framework, Bitable checkpointer) | [docs.rs](https://docs.rs/synaptic-lark) |
| `synaptic` | Unified facade crate that re-exports all sub-crates under a single namespace | [docs.rs](https://docs.rs/synaptic) |

> **Note:** The docs.rs links above will become active once the crates are published to crates.io. In the meantime, generate local documentation as described below.

## Local API Documentation

You can generate and browse the full API documentation locally with:

```bash
cargo doc --workspace --open
```

This builds rustdoc for every crate in the workspace and opens the result in your browser. The generated documentation includes all public types, traits, functions, and their doc comments.

To generate docs without opening the browser (useful in CI):

```bash
cargo doc --workspace --no-deps
```

## Using the Facade Crate

If you prefer a single dependency instead of listing individual crates, use the `synaptic` facade:

```toml
[dependencies]
synaptic = "0.4"
```

Then import through the unified namespace:

```rust
use synaptic::core::Message;
use synaptic::openai::OpenAiChatModel;   // requires "openai" feature
use synaptic::models::ScriptedChatModel; // requires "model-utils" feature
use synaptic::graph::create_react_agent;
use synaptic::runnables::Runnable;
```
