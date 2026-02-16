# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Synapse is a Rust agent framework with LangChain-compatible architecture. It provides composable building blocks for AI agents: tool execution, memory, callbacks, retrieval, and evaluation. Phase 1 (core refactor) is complete; Phase 2 (multi-provider model adapters + streaming) is next.

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

13 library crates in `crates/`, 3 example binaries in `examples/`:

**Core layer** — `synapse-core` defines all shared traits and types:
- `ChatModel`, `Tool`, `MemoryStore`, `CallbackHandler`, `Agent` traits
- `Message` enum (`System`, `Human`, `AI`, `Tool` variants) — factory methods: `Message::system()`, `human()`, `ai()`, `ai_with_tool_calls()`, `tool()`; accessors: `content()`, `role()`, `is_*()`, `tool_calls()`, `tool_call_id()`
- `AIMessageChunk` — streaming chunk with `+`/`+=` merge and `into_message()`
- `ChatRequest`, `ChatResponse` (message + usage), `ToolCall`, `RunEvent`, `TokenUsage`
- `RunnableConfig` — runtime config (tags, metadata, max_concurrency, recursion_limit, run_id, run_name)
- `SynapseError` enum (19 variants covering all subsystems)

**Implementation crates** — each implements one core trait:
- `synapse-agents` — `ReActAgentExecutor` (think → tool → observe loop, max_steps guard)
- `synapse-tools` — `ToolRegistry` (Arc<RwLock<HashMap>>) + `SerialToolExecutor`
- `synapse-memory` — `InMemoryStore` (session-keyed message storage)
- `synapse-callbacks` — `RecordingCallback`, `LoggingCallback`
- `synapse-models` — `ScriptedChatModel` (test double; real adapters are Phase 2)
- `synapse-prompts` — `PromptTemplate` with `{{ variable }}` interpolation

**Composition & retrieval crates:**
- `synapse-runnables` — `Runnable<I, O>` trait + `IdentityRunnable`
- `synapse-chains` — `SequentialChain` (pipes runnable outputs)
- `synapse-retrieval` — `Retriever` trait + `InMemoryRetriever`; `Document` has `id`, `content`, `metadata: HashMap<String, Value>`
- `synapse-loaders` — `TextLoader` (wraps text into `Document`)
- `synapse-guardrails` — `JsonObjectGuard` (validates JSON shape)
- `synapse-eval` — `EvalCase`/`EvalReport` (accuracy metrics)

## Key Patterns

- **Message is a tagged enum** — `#[serde(tag = "role")]` with variants `System`, `Human`, `AI` (carries `tool_calls`), `Tool` (carries `tool_call_id`). Use factory methods, not struct literals.
- **All traits are async** via `#[async_trait]`. Tests use `#[tokio::test]`.
- **Concurrency**: `Arc<RwLock<_>>` for registries, `Arc<tokio::sync::Mutex<_>>` for callbacks/memory.
- **Session isolation**: Memory and agent runs are keyed by `session_id`.
- **Event-driven callbacks**: `RunEvent` enum fired at each agent lifecycle stage.
- **Each crate has `tests/` directory** with integration-style tests in separate files.

## Workspace Dependencies (shared via `[workspace.dependencies]`)

async-trait, serde/serde_json, thiserror 2.0, tokio (macros + rt-multi-thread + sync + time), tracing/tracing-subscriber. Rust edition 2021, MSRV 1.78.

## Development Roadmap

Full 12-phase plan in `docs/plans/2026-02-16-synapse-full-langchain-parity-design.md`. Phase 1 implementation details in `docs/plans/2026-02-16-phase1-core-refactor.md`.
