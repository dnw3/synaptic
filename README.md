# Synaptic

[![CI](https://github.com/dnw3/synaptic/actions/workflows/ci.yml/badge.svg)](https://github.com/dnw3/synaptic/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/synaptic.svg)](https://crates.io/crates/synaptic)
[![docs.rs](https://docs.rs/synaptic/badge.svg)](https://docs.rs/synaptic)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.83-orange.svg)](https://blog.rust-lang.org/2024/11/28/Rust-1.83.0.html)

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

Add `synaptic` to your `Cargo.toml`:

```toml
[dependencies]
synaptic = "0.1"
```

By default, all features are enabled. You can select specific features:

```toml
[dependencies]
synaptic = { version = "0.1", default-features = false, features = ["models", "runnables", "graph"] }
```

Available features: `models`, `runnables`, `prompts`, `parsers`, `tools`, `memory`, `callbacks`, `graph`, `retrieval`, `loaders`, `splitters`, `embeddings`, `vectorstores`, `cache`, `eval`.

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

17 library crates in `crates/`, 13 examples in `examples/`:

| Crate | Description |
|-------|-------------|
| `synaptic-core` | Shared traits and types: `ChatModel`, `Message`, `ToolChoice`, `SynapticError` |
| `synaptic-models` | LLM provider adapters + retry/rate-limit/structured-output wrappers |
| `synaptic-runnables` | LCEL: `Runnable`, `BoxRunnable`, pipe, Lambda, Parallel, Branch, Assign, Pick, Fallbacks |
| `synaptic-prompts` | `ChatPromptTemplate`, `FewShotChatMessagePromptTemplate` |
| `synaptic-parsers` | Str, Json, Structured, List, Enum output parsers |
| `synaptic-tools` | `ToolRegistry`, `SerialToolExecutor` |
| `synaptic-memory` | Buffer, Window, Summary, SummaryBuffer, TokenBuffer, `RunnableWithMessageHistory` |
| `synaptic-callbacks` | `RecordingCallback`, `TracingCallback`, `CompositeCallback` |
| `synaptic-retrieval` | BM25, MultiQuery, Ensemble, Compression, SelfQuery, ParentDocument retrievers |
| `synaptic-loaders` | Text, JSON, CSV, Directory document loaders |
| `synaptic-splitters` | Character, Recursive, Markdown, Token text splitters |
| `synaptic-embeddings` | `Embeddings` trait, Fake, OpenAI, Ollama providers |
| `synaptic-vectorstores` | `VectorStore` trait, InMemory (cosine), `VectorStoreRetriever` |
| `synaptic-graph` | `StateGraph`, `CompiledGraph` (with stream), `ToolNode`, `create_react_agent` |
| `synaptic-cache` | InMemory, Semantic caches, `CachedChatModel` |
| `synaptic-eval` | `Evaluator` trait, 5 evaluators, `Dataset`, batch `evaluate()` |
| `synaptic` | Unified facade re-exporting all crates |

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

- **Book**: [dnw3.github.io/synaptic](https://dnw3.github.io/synaptic) — tutorials, how-to guides, concepts
- **API Reference**: [docs.rs/synaptic](https://docs.rs/synaptic) — full Rustdoc API documentation

## Design Principles

- Core abstractions first, feature crates expanded incrementally
- LangChain concept compatibility with Rust-idiomatic APIs
- All traits are async via `#[async_trait]`, runtime is tokio
- Type-erased composition via `BoxRunnable` with `|` pipe operator
- `Arc<RwLock<_>>` for shared registries, session-keyed memory isolation

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, or the [full guide](https://dnw3.github.io/synaptic/contributing.html).

## License

MIT — see [LICENSE](LICENSE) for details.
