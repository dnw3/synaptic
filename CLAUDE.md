# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Synapse is a Rust agent framework with LangChain-compatible architecture. It provides composable building blocks for AI agents: tool execution, memory, callbacks, retrieval, and evaluation. Phases 1-12 complete (core through structured output). Phase 13 complete: LangChain alignment — tool_choice, LCEL streaming + bind + assign/pick, graph streaming, TokenTextSplitter, ConversationSummaryBufferMemory, SelfQueryRetriever, ParentDocumentRetriever, deprecations, cleanup.

## Build & Test Commands

```bash
cargo build --workspace              # Build all crates
cargo test --workspace               # Run all tests (all should pass)
cargo test -p synapse-tools          # Test a single crate
cargo test -p synapse-core -- chunk  # Run specific test by name pattern
cargo run -p tool_calling_basic      # Run example binary
cargo clippy --workspace             # Lint
cargo fmt --all -- --check           # Check formatting
```

## Workspace Architecture

18 library crates in `crates/` (including `synapse` facade), 3 example binaries in `examples/`:

**Core layer** — `synapse-core` defines all shared traits and types:
- `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler` traits
- `Message` enum (`System`, `Human`, `AI`, `Tool` variants) — factory methods: `Message::system()`, `human()`, `ai()`, `ai_with_tool_calls()`, `tool()`; accessors: `content()`, `role()`, `is_*()`, `tool_calls()`, `tool_call_id()`
- `AIMessageChunk` — streaming chunk with `+`/`+=` merge and `into_message()`
- `ChatRequest` (messages + tools + tool_choice), `ChatResponse` (message + usage), `ToolCall`, `ToolDefinition`, `ToolChoice` (Auto/Required/None/Specific), `RunEvent`, `TokenUsage`
- `ChatStream` type alias — `Pin<Box<dyn Stream<Item = Result<AIMessageChunk, SynapseError>> + Send>>`
- `RunnableConfig` — runtime config (tags, metadata, max_concurrency, recursion_limit, run_id, run_name)
- `SynapseError` enum (19 variants covering all subsystems)

**Implementation crates** — each implements one core trait:
- `synapse-tools` — `ToolRegistry` (Arc<RwLock<HashMap>>) + `SerialToolExecutor`
- `synapse-memory` — `InMemoryStore` (session-keyed message storage), memory strategies: `ConversationBufferMemory`, `ConversationWindowMemory` (last K messages), `ConversationSummaryMemory` (LLM summarization), `ConversationTokenBufferMemory` (token budget), `ConversationSummaryBufferMemory` (hybrid summary + recent buffer with token threshold), `RunnableWithMessageHistory` (auto load/save wrapper)
- `synapse-callbacks` — `RecordingCallback`, `TracingCallback` (structured tracing spans/events), `CompositeCallback` (multi-handler dispatch)
- `synapse-models` — provider adapters (`OpenAiChatModel`, `AnthropicChatModel`, `GeminiChatModel`, `OllamaChatModel`) + `ScriptedChatModel` (test double) + `RetryChatModel`, `RateLimitedChatModel`, `TokenBucketChatModel` wrappers + `StructuredOutputChatModel<T>` (JSON schema enforcement) + `ProviderBackend` trait (`HttpBackend`, `FakeBackend`)
- `synapse-cache` — `LlmCache` trait, `InMemoryCache` (optional TTL), `SemanticCache` (embedding similarity matching), `CachedChatModel` (wraps ChatModel with cache)
- `synapse-prompts` — `PromptTemplate` (`{{ variable }}` interpolation), `ChatPromptTemplate` (produces `Vec<Message>` with `MessageTemplate` variants: System/Human/AI/Placeholder), `FewShotChatMessagePromptTemplate` (example-based prompting); all chat templates implement `Runnable`
- `synapse-parsers` — output parsers, all implement `Runnable`: `StrOutputParser` (Message→String), `JsonOutputParser` (String→Value), `StructuredOutputParser<T>` (String→T via serde), `ListOutputParser` (String→Vec<String>, configurable separator), `EnumOutputParser` (validates against allowed values)

**Composition & retrieval crates:**
- `synapse-runnables` — `Runnable<I, O>` trait with `invoke()`/`batch()`/`stream()`/`boxed()`, `BoxRunnable` (type-erased, `|` pipe operator via `BitOr`, `bind()` for config transforms), composition types: `RunnablePassthrough`, `RunnableLambda`, `RunnableSequence`, `RunnableParallel`, `RunnableBranch`, `RunnableWithFallbacks`, `RunnableAssign` (parallel branch merge into JSON), `RunnablePick` (key extraction from JSON)
- `synapse-retrieval` — `Retriever` trait, `InMemoryRetriever`, `BM25Retriever` (Okapi BM25 scoring), `MultiQueryRetriever` (LLM-generated query variants), `EnsembleRetriever` (Reciprocal Rank Fusion), `ContextualCompressionRetriever` + `DocumentCompressor` trait + `EmbeddingsFilter` (similarity threshold filtering), `SelfQueryRetriever` (LLM-powered metadata filtering), `ParentDocumentRetriever` (child→parent mapping); `Document` has `id`, `content`, `metadata: HashMap<String, Value>`
- `synapse-loaders` — `Loader` async trait, `TextLoader`, `JsonLoader` (configurable id/content keys), `CsvLoader` (column-based with metadata), `DirectoryLoader` (glob filtering, recursive)
- `synapse-splitters` — `TextSplitter` trait with `split_text()`/`split_documents()`, `CharacterTextSplitter`, `RecursiveCharacterTextSplitter` (hierarchical separators), `MarkdownHeaderTextSplitter` (header-aware with metadata), `TokenTextSplitter` (~4 chars/token heuristic, word-boundary splitting)
- `synapse-embeddings` — `Embeddings` trait (`embed_documents`/`embed_query`), `FakeEmbeddings` (deterministic testing), `OpenAiEmbeddings`, `OllamaEmbeddings` (use `ProviderBackend`)
- `synapse-vectorstores` — `VectorStore` trait (`add_documents`/`similarity_search`/`delete`), `InMemoryVectorStore` (cosine similarity, `RwLock<HashMap>`), `VectorStoreRetriever` (bridges to `Retriever` trait)
- `synapse-graph` — LangGraph-style state machine: `State` trait (merge/reduce), `MessageState`, `Node<S>` trait + `FnNode`, `StateGraph<S>` builder (add_node/add_edge/add_conditional_edges/interrupt_before/interrupt_after/compile), `CompiledGraph<S>` (invoke/invoke_with_config/stream/stream_with_config/update_state), `StreamMode` (Values/Updates), `GraphEvent<S>`, `Checkpointer` trait + `MemorySaver`, `ToolNode`, `create_react_agent(model, tools)`
- `synapse` — unified facade crate re-exporting all sub-crates (`use synapse::core::Message`, `use synapse::models::OpenAiChatModel`, etc.)
- `synapse-eval` — `Evaluator` trait + `EvalResult`, evaluators: `ExactMatchEvaluator`, `JsonValidityEvaluator`, `RegexMatchEvaluator`, `EmbeddingDistanceEvaluator`, `LLMJudgeEvaluator`; `Dataset` + `evaluate()` batch pipeline; legacy `EvalCase`/`EvalReport`

## Key Patterns

- **Message is a tagged enum** — `#[serde(tag = "role")]` with variants `System`, `Human`, `AI` (carries `tool_calls`), `Tool` (carries `tool_call_id`). Use factory methods, not struct literals.
- **ChatModel has streaming** — `chat()` for single response, `stream_chat()` returns `ChatStream` (default impl wraps `chat()` as single chunk).
- **ChatRequest uses constructor** — `ChatRequest::new(messages)`, optional `.with_tools(tools)`, `.with_tool_choice(choice)`. Never use struct literal.
- **Provider adapters use ProviderBackend** — `HttpBackend` for production, `FakeBackend` for tests. Adapters map Synapse types ↔ provider JSON.
- **All traits are async** via `#[async_trait]`. Tests use `#[tokio::test]`.
- **Concurrency**: `Arc<RwLock<_>>` for registries, `Arc<tokio::sync::Mutex<_>>` for callbacks/memory.
- **Session isolation**: Memory and agent runs are keyed by `session_id`.
- **Event-driven callbacks**: `RunEvent` enum fired at each agent lifecycle stage.
- **LCEL pipe composition** — `let chain = step1.boxed() | step2.boxed();` via `BitOr` on `BoxRunnable`. `RunnableLambda::new(|x| async { Ok(transform(x)) })` wraps async closures. `RunnableParallel` runs named branches concurrently, merges to `serde_json::Value`. `RunnableBranch` routes by condition with default fallthrough. `BoxRunnable::bind()` applies config transforms. `Runnable::stream()` yields results through pipelines (final component truly streams).
- **Graph streaming** — `graph.stream(state, StreamMode::Values)` yields `GraphEvent { node, state }` after each node execution. `StreamMode::Updates` yields per-node state changes.
- **Each crate has `tests/` directory** with integration-style tests in separate files.

## Workspace Dependencies (shared via `[workspace.dependencies]`)

async-trait, serde/serde_json, thiserror 2.0, tokio (macros + rt-multi-thread + sync + time), tracing/tracing-subscriber, reqwest (json + stream), futures, async-stream, eventsource-stream, bytes, csv, regex. Rust edition 2021, MSRV 1.78.

## Development Roadmap

Full 12-phase plan in `docs/plans/2026-02-16-synapse-full-langchain-parity-design.md`. Phase 1 implementation details in `docs/plans/2026-02-16-phase1-core-refactor.md`. Phase 13 (LangChain alignment) gap analysis in the plan mode file.
