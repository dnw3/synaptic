# Synapse

Synapse is a Rust agent framework designed with LangChain-compatible architecture principles and Rust-native interfaces.

## Current Status

This repository currently includes:

- Phase 0 foundation (workspace + quality gates)
- Phase 1 MVP core (ReAct loop + tools + memory + callbacks)

## Workspace Layout

- `crates/synapse-core`: core traits and shared types
- `crates/synapse-prompts`: prompt templating
- `crates/synapse-tools`: tool registry and serial executor
- `crates/synapse-memory`: in-memory conversation storage
- `crates/synapse-callbacks`: callback implementations (recording/logging)
- `crates/synapse-models`: scripted model for testing/examples
- `crates/synapse-agents`: ReAct agent executor
- `crates/synapse-runnables`: generic runnable abstraction
- `crates/synapse-chains`: sequential chain composition
- `crates/synapse-retrieval`: document + retriever abstraction
- `crates/synapse-loaders`: document loading helpers
- `crates/synapse-guardrails`: JSON object validation guardrail
- `crates/synapse-eval`: basic evaluation report metrics
- `examples/react_basic`: end-to-end ReAct example
- `examples/tool_calling_basic`: direct tool execution example
- `examples/memory_chat`: memory usage example

## Quick Start

```bash
cargo test --workspace
cargo run -p tool_calling_basic
cargo run -p memory_chat
cargo run -p react_basic
```

## Design Direction

- Core abstraction first, feature crates expanded incrementally
- LangChain concept compatibility, Rust idiomatic API design
- Default deterministic serial tool execution with extension points for concurrency
- Memory abstraction for future persistent backends
