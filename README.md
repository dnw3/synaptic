# Synapse

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

## Workspace Layout

18 library crates in `crates/`, 3 examples in `examples/`:

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

## Quick Start

```bash
# Build & test
cargo build --workspace
cargo test --workspace

# Run examples
cargo run -p tool_calling_basic   # Tool registry and execution
cargo run -p memory_chat          # Session-based conversation memory
cargo run -p react_basic          # ReAct agent with tool calling
```

## Usage

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

See [`examples/`](examples/) for complete runnable demos.

## Design Principles

- Core abstractions first, feature crates expanded incrementally
- LangChain concept compatibility with Rust-idiomatic APIs
- All traits are async via `#[async_trait]`, runtime is tokio
- Type-erased composition via `BoxRunnable` with `|` pipe operator
- `Arc<RwLock<_>>` for shared registries, session-keyed memory isolation
