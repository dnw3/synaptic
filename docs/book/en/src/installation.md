# Installation

## Requirements

- **Rust edition**: 2021
- **Minimum supported Rust version (MSRV)**: 1.88
- **Runtime**: Tokio (async runtime)

## Adding Synaptic to Your Project

The `synaptic` facade crate re-exports all sub-crates. Use **feature flags** to control which modules are compiled.

### Feature Flags

Synaptic provides fine-grained feature flags, similar to tokio:

```toml
[dependencies]
# Full — everything enabled
synaptic = { version = "0.4", features = ["full"] }

# Agent development (tools + graph + memory + middleware + your chosen provider)
synaptic = { version = "0.4", features = ["openai", "agent"] }

# RAG applications (retrieval + loaders + splitters + embeddings + vectorstores)
synaptic = { version = "0.4", features = ["openai", "rag"] }

# Agent + RAG
synaptic = { version = "0.4", features = ["agent", "rag"] }

# Just OpenAI model calls
synaptic = { version = "0.4", features = ["openai"] }

# All providers
synaptic = { version = "0.4", features = ["models"] }

# Fine-grained: one provider + specific modules
synaptic = { version = "0.4", features = ["anthropic", "graph", "middleware"] }
```

**Composite features:**

| Feature | Description |
|---------|-------------|
| **`default`** | `runnables`, `prompts`, `parsers`, `tools`, `callbacks` |
| **`agent`** | `default` + `openai`, `graph`, `memory`, `middleware`, `store`, `condenser`, `secrets`, `config`, `session` |
| **`rag`** | `default` + `openai`, `embeddings`, `retrieval`, `loaders`, `splitters`, `vectorstores` |
| **`models`** | All providers: `openai` + `anthropic` + `gemini` + `ollama` + `bedrock` + `cohere` |
| **`full`** | All features enabled (all providers, integrations, otel, langfuse, store-filesystem, deep-config) |

**Provider features** (each enables one provider within `synaptic-models`):

| Feature | Description |
|---------|-------------|
| `openai` | OpenAI (`OpenAiChatModel`, `OpenAiEmbeddings`) |
| `anthropic` | Anthropic (`AnthropicChatModel`) |
| `gemini` | Google Gemini (`GeminiChatModel`) |
| `ollama` | Ollama (`OllamaChatModel`, `OllamaEmbeddings`) |
| `bedrock` | AWS Bedrock (`BedrockChatModel`) |
| `cohere` | Cohere (`CohereReranker`) |

OpenAI-compatible providers (Groq, DeepSeek, Mistral, Together, Fireworks, xAI, Perplexity) are enabled via their respective feature flags: `groq`, `deepseek`, `mistral`, `together`, `fireworks`, `xai`, `perplexity`.

**Module features:**

| Feature | Description |
|---------|-------------|
| `graph` | Graph orchestration (`StateGraph`, `create_react_agent`, `InterceptorChain`) |
| `middleware` | Interceptor chain (tool call limits, HITL, summarization, SSRF guard, circuit breaker) |
| `memory` | Memory strategies (buffer, window, summary, token buffer) |
| `store` | Persistence backends (postgres, redis, sqlite, mongodb) |
| `mcp` | Model Context Protocol client (Stdio/SSE/HTTP transports) |
| `macros` | Proc macros (`#[tool]`, `#[chain]`, `#[entrypoint]`, `#[traceable]`) |
| `deep` | Deep Agent harness (ACP protocol, built-in tools, sub-agents, skills) |
| `events` | EventBus with 29 event kinds and 5 dispatch modes |
| `config` | Agent config loading + secrets masking + plugin system |

**Integration features:**

| Feature | Description |
|---------|-------------|
| `qdrant` | Qdrant vector store (via `synaptic-rag`) |
| `postgres` | PostgreSQL store, cache, vector store, checkpointer (via `synaptic-store`) |
| `redis` | Redis store + cache (via `synaptic-store`) |
| `sqlite` | SQLite store (via `synaptic-store`) |
| `mongodb` | MongoDB store (via `synaptic-store`) |
| `pinecone` | Pinecone vector store (via `synaptic-rag`) |
| `chroma` | Chroma vector store (via `synaptic-rag`) |
| `elasticsearch` | Elasticsearch vector store (via `synaptic-rag`) |
| `opensearch` | OpenSearch vector store (via `synaptic-rag`) |
| `milvus` | Milvus vector store (via `synaptic-rag`) |
| `lancedb` | LanceDB vector store (via `synaptic-rag`) |
| `weaviate` | Weaviate vector store (via `synaptic-rag`) |
| `pdf` | PDF document loader (via `synaptic-tools`) |
| `tavily` | Tavily search tool (via `synaptic-integrations`) |
| `confluence` | Confluence integration (via `synaptic-integrations`) |
| `slack` | Slack integration (via `synaptic-integrations`) |
| `lark` | Lark/Feishu bot framework (via `synaptic-lark`) |
| `otel` | OpenTelemetry tracing |
| `langfuse` | Langfuse observability |

The `core` module (traits and types) is always available regardless of feature selection.

### Quick Start Example

```toml
[dependencies]
synaptic = { version = "0.4", features = ["openai", "agent"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Using the Facade

The facade crate provides namespaced re-exports for all sub-crates:

```rust
use synaptic::core::{ChatModel, ChatRequest, ChatResponse, Message, SynapticError};
use synaptic::models::{OpenAiChatModel, AnthropicChatModel};  // requires provider features
use synaptic::graph::{StateGraph, create_react_agent};
use synaptic::rag::{Retriever, InMemoryVectorStore, RecursiveCharacterTextSplitter};
use synaptic::middleware::{Interceptor, InterceptorChain};
```

Alternatively, you can depend on individual crates directly if you want to minimize compile times:

```toml
[dependencies]
synaptic-core = "0.4"
synaptic-models = { version = "0.4", features = ["openai"] }
synaptic-graph = "0.4"
```

## Provider API Keys

Synaptic reads API keys from environment variables. Set the ones you need for your chosen provider:

| Provider | Environment Variable |
|---|---|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Google Gemini | `GOOGLE_API_KEY` |
| Ollama | No key required (runs locally) |

For example, on a Unix shell:

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="AI..."
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
cargo test -p synaptic-models

# Run a specific test by name
cargo test -p synaptic-core -- trim_messages

# Check formatting
cargo fmt --all -- --check

# Run lints
cargo clippy --workspace
```

## Workspace Dependencies

Synaptic uses Cargo workspace-level dependency management. Key shared dependencies include:

- `async-trait` -- async trait methods
- `serde` / `serde_json` -- serialization
- `thiserror` 2.0 -- error derive
- `tokio` -- async runtime (macros, rt-multi-thread, sync, time)
- `reqwest` -- HTTP client (json, stream features)
- `futures` / `async-stream` -- stream utilities
- `tracing` / `tracing-subscriber` -- structured logging
