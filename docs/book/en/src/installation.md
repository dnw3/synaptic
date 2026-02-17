# Installation

## Requirements

- **Rust edition**: 2021
- **Minimum supported Rust version (MSRV)**: 1.78
- **Runtime**: Tokio (async runtime)

## Adding Synapse to Your Project

The `synapse` facade crate re-exports all sub-crates, so a single dependency gives you access to everything.

**From a local checkout** (during development or before the crate is published):

```toml
[dependencies]
synapse = { path = "path/to/synapse/crates/synapse" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

**Once published to crates.io**:

```toml
[dependencies]
synapse = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Using the Facade

The facade crate provides namespaced re-exports for all sub-crates. You access types through their module path:

```rust
use synapse::core::{ChatModel, ChatRequest, ChatResponse, Message, SynapseError};
use synapse::models::{OpenAiChatModel, ScriptedChatModel};
use synapse::runnables::{Runnable, BoxRunnable, RunnableLambda};
use synapse::prompts::ChatPromptTemplate;
use synapse::parsers::StrOutputParser;
use synapse::tools::ToolRegistry;
use synapse::memory::InMemoryStore;
use synapse::graph::{StateGraph, create_react_agent};
use synapse::retrieval::Retriever;
use synapse::embeddings::OpenAiEmbeddings;
use synapse::vectorstores::InMemoryVectorStore;
```

Alternatively, you can depend on individual crates directly if you want to minimize compile times:

```toml
[dependencies]
synapse-core = { path = "path/to/synapse/crates/synapse-core" }
synapse-models = { path = "path/to/synapse/crates/synapse-models" }
```

## Provider API Keys

Synapse reads API keys from environment variables. Set the ones you need for your chosen provider:

| Provider | Environment Variable |
|---|---|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Ollama | No key required (runs locally) |

For example, on a Unix shell:

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="AI..."
```

You do not need any API keys to run the [Quickstart](quickstart.md) example, which uses the `ScriptedChatModel` test double.

## Building and Testing

From the workspace root:

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Test a single crate
cargo test -p synapse-models

# Run a specific test by name
cargo test -p synapse-core -- trim_messages

# Check formatting
cargo fmt --all -- --check

# Run lints
cargo clippy --workspace
```

## Workspace Dependencies

Synapse uses Cargo workspace-level dependency management. Key shared dependencies include:

- `async-trait` -- async trait methods
- `serde` / `serde_json` -- serialization
- `thiserror` 2.0 -- error derive
- `tokio` -- async runtime (macros, rt-multi-thread, sync, time)
- `reqwest` -- HTTP client (json, stream features)
- `futures` / `async-stream` -- stream utilities
- `tracing` / `tracing-subscriber` -- structured logging
