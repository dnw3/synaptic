# Synapse

[![CI](https://github.com/AIMOverse/synapse/actions/workflows/ci.yml/badge.svg)](https://github.com/AIMOverse/synapse/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/synapse.svg)](https://crates.io/crates/synapse)
[![docs.rs](https://docs.rs/synapse/badge.svg)](https://docs.rs/synapse)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-orange.svg)](https://blog.rust-lang.org/2024/05/02/Rust-1.78.0.html)

A Rust agent framework with LangChain-compatible architecture and Rust-native async interfaces.

## Features

- **LLM Adapters** — OpenAI, Anthropic, Gemini, Ollama with streaming, retry, rate limiting
- **LCEL Composition** — `Runnable` trait with pipe operator (`|`), streaming, bind, parallel, branch, fallbacks
- **Graph Orchestration** — LangGraph-style `StateGraph` with conditional edges, checkpointing, human-in-the-loop, streaming
- **ReAct Agent** — `create_react_agent(model, tools)` with automatic tool dispatch
- **Tool System** — `Tool` trait, `ToolRegistry`, `SerialToolExecutor`, `tool_choice` control
- **Memory** — Buffer, Window, Summary, SummaryBuffer, TokenBuffer strategies
- **Prompt Templates** — Chat templates, few-shot prompting, placeholder interpolation
- **Output Parsers** — String, JSON, Structured<T>, List, Enum — all composable as `Runnable`
- **RAG Pipeline** — Document loaders, text splitters, embeddings, vector stores, 7 retriever types
- **Caching** — In-memory, semantic (embedding similarity), `CachedChatModel` wrapper
- **Evaluation** — ExactMatch, JsonValidity, Regex, EmbeddingDistance, LLMJudge evaluators
- **Structured Output** — `StructuredOutputChatModel<T>` with JSON schema enforcement
- **Observability** — `TracingCallback` (structured spans), `CompositeCallback` (multi-handler)

## Installation

Add `synapse` to your `Cargo.toml`:

```toml
[dependencies]
synapse = "0.1"
```

By default, all features are enabled. You can select specific features:

```toml
[dependencies]
synapse = { version = "0.1", default-features = false, features = ["models", "runnables", "graph"] }
```

Available features: `models`, `runnables`, `prompts`, `parsers`, `tools`, `memory`, `callbacks`, `graph`, `retrieval`, `loaders`, `splitters`, `embeddings`, `vectorstores`, `cache`, `eval`.

## Quick Start

```rust
use synapse::core::{ChatModel, Message, ChatRequest, ToolChoice};
use synapse::runnables::{Runnable, RunnableLambda};
use synapse::graph::{create_react_agent, MessageState};

// LCEL pipe composition
let chain = step1.boxed() | step2.boxed() | step3.boxed();
let result = chain.invoke(input, &config).await?;

// ReAct agent
let graph = create_react_agent(model, tools)?;
let state = MessageState { messages: vec![Message::human("Hello")] };
let result = graph.invoke(state).await?;
```

## Workspace Layout

17 library crates in `crates/`, 13 examples in `examples/`:

| Crate | Description |
|-------|-------------|
| `synapse-core` | Shared traits and types: `ChatModel`, `Message`, `ToolChoice`, `SynapseError` |
| `synapse-models` | LLM provider adapters + retry/rate-limit/structured-output wrappers |
| `synapse-runnables` | LCEL: `Runnable`, `BoxRunnable`, pipe, Lambda, Parallel, Branch, Assign, Pick, Fallbacks |
| `synapse-prompts` | `ChatPromptTemplate`, `FewShotChatMessagePromptTemplate` |
| `synapse-parsers` | Str, Json, Structured, List, Enum output parsers |
| `synapse-tools` | `ToolRegistry`, `SerialToolExecutor` |
| `synapse-memory` | Buffer, Window, Summary, SummaryBuffer, TokenBuffer, `RunnableWithMessageHistory` |
| `synapse-callbacks` | `RecordingCallback`, `TracingCallback`, `CompositeCallback` |
| `synapse-retrieval` | BM25, MultiQuery, Ensemble, Compression, SelfQuery, ParentDocument retrievers |
| `synapse-loaders` | Text, JSON, CSV, Directory document loaders |
| `synapse-splitters` | Character, Recursive, Markdown, Token text splitters |
| `synapse-embeddings` | `Embeddings` trait, Fake, OpenAI, Ollama providers |
| `synapse-vectorstores` | `VectorStore` trait, InMemory (cosine), `VectorStoreRetriever` |
| `synapse-graph` | `StateGraph`, `CompiledGraph` (with stream), `ToolNode`, `create_react_agent` |
| `synapse-cache` | InMemory, Semantic caches, `CachedChatModel` |
| `synapse-eval` | `Evaluator` trait, 5 evaluators, `Dataset`, batch `evaluate()` |
| `synapse` | Unified facade re-exporting all crates |

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
```

All examples use `ScriptedChatModel` and `FakeEmbeddings` — no API keys required.

## Documentation

- **Book**: [aimoverse.github.io/synapse](https://aimoverse.github.io/synapse) — tutorials, how-to guides, concepts
- **API Reference**: [docs.rs/synapse](https://docs.rs/synapse) — full Rustdoc API documentation

## Design Principles

- Core abstractions first, feature crates expanded incrementally
- LangChain concept compatibility with Rust-idiomatic APIs
- All traits are async via `#[async_trait]`, runtime is tokio
- Type-erased composition via `BoxRunnable` with `|` pipe operator
- `Arc<RwLock<_>>` for shared registries, session-keyed memory isolation

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, or the [full guide](https://aimoverse.github.io/synapse/contributing.html).

## License

MIT — see [LICENSE](LICENSE) for details.
