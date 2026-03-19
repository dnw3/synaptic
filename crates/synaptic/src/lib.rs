//! Synaptic — A Rust agent framework with LangChain-compatible architecture.
//!
//! This crate re-exports all Synaptic sub-crates for convenient single-import usage.
//! Enable features to control which modules are available.
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `default` | `runnables`, `prompts`, `parsers`, `tools` |
//! | `model-utils` | `ProviderBackend`, `ScriptedChatModel`, wrappers (Retry, RateLimit, etc.) |
//! | `openai` | OpenAI ChatModel + Embeddings |
//! | `anthropic` | Anthropic ChatModel (via synaptic-models) |
//! | `gemini` | Gemini ChatModel (via synaptic-models) |
//! | `ollama` | Ollama ChatModel + Embeddings (via synaptic-models) |
//! | `models` | All providers: openai + anthropic + gemini + ollama + bedrock + cohere |
//! | `agent` | Agent capabilities (graph, memory, middleware, store, etc.) — no provider included |
//! | `rag` | RAG pipeline (embeddings, retrieval, loaders, etc.) — no provider included |
//! | `agent-openai` | `agent` + openai provider |
//! | `agent-anthropic` | `agent` + anthropic provider |
//! | `rag-openai` | `rag` + openai provider |
//! | `deep` | `agent` + deep agent harness (no implicit provider) |
//! | `deep-config` | `deep` + config + openai (config-builder requires openai) |
//! | `full` | All features enabled |
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use synaptic::core::{ChatModel, Message, ChatRequest, ToolChoice};
//! use synaptic::openai::OpenAiChatModel;
//! use synaptic::core::runnable::{Runnable, RunnableLambda, RunnableAssign, RunnablePick};
//! ```

// Re-export internal crates under their original names so proc-macro generated code
// (which references `::synaptic::synaptic_core`, etc. via `proc-macro-crate` detection)
// can resolve correctly when downstream crates only depend on the `synaptic` facade.
#[doc(hidden)]
pub extern crate synaptic_core;
#[cfg(feature = "middleware")]
#[doc(hidden)]
pub extern crate synaptic_middleware;

/// Core traits and types: ChatModel, Message, ToolChoice, SynapticError, RunnableConfig, etc.
/// Always available.
pub use synaptic_core as core;

/// Unified routing metadata for cross-channel message delivery.
pub use synaptic_core::DeliveryContext;

/// DM access control policy and enforcement.
pub use synaptic_core::{
    DmAccessDenied, DmPolicy, DmPolicyEnforcer, PairingChallenge, PairingError,
};

/// Message origin tracking for auditing and routing decisions.
pub use synaptic_core::{InputProvenance, ProvenanceKind};

/// Channel connection status tracking types.
pub use synaptic_core::{
    ChannelAccountSnapshot, ChannelProbe, ChannelState, ChannelStatusHandle, ChannelStatusPatch,
    DisconnectInfo,
};

/// LCEL composition: Runnable trait (with stream), BoxRunnable (with bind), pipe operator,
/// Lambda, Parallel, Branch, Assign, Pick, Fallbacks, etc.
/// Now consolidated into synaptic-core.
#[cfg(feature = "runnables")]
pub use synaptic_core::runnable as runnables;

/// ProviderBackend abstraction, ScriptedChatModel, and ChatModel wrappers
/// (Retry, RateLimit, TokenBucket, StructuredOutput, BoundTools).
#[cfg(feature = "model-utils")]
pub use synaptic_models as models;

/// OpenAI ChatModel and Embeddings.
#[cfg(feature = "openai")]
pub mod openai {
    pub use synaptic_models::openai::*;
}

/// Prompt templates: ChatPromptTemplate, FewShotChatMessagePromptTemplate.
#[cfg(feature = "prompts")]
pub use synaptic_rag::prompts;

/// Output parsers: Str, Json, Structured, List, Enum.
#[cfg(feature = "parsers")]
pub use synaptic_rag::parsers;

/// Tool registry and execution.
#[cfg(feature = "tools")]
pub use synaptic_tools as tools;

/// Memory strategies: Buffer, Window, Summary, SummaryBuffer, TokenBuffer, RunnableWithMessageHistory.
#[cfg(feature = "memory")]
pub use synaptic_memory as memory;

/// Retrieval: Retriever trait, BM25, MultiQuery, Ensemble, Compression, SelfQuery, ParentDocument, Document.
#[cfg(feature = "retrieval")]
pub use synaptic_rag::retrieval;

/// Document loaders: Text, JSON, CSV, Directory.
#[cfg(feature = "loaders")]
pub use synaptic_rag::loaders;

/// Text splitters: Character, Recursive, Markdown, Token.
#[cfg(feature = "splitters")]
pub use synaptic_rag::splitters;

/// Embeddings: trait, Fake, CacheBacked, and provider implementations.
#[cfg(feature = "embeddings")]
pub use synaptic_rag::embeddings;

/// Vector stores: InMemory, VectorStoreRetriever, and provider implementations.
#[cfg(feature = "vectorstores")]
pub use synaptic_rag::vectorstores;

/// Graph agent orchestration: StateGraph, CompiledGraph (with stream), GraphEvent, StreamMode, checkpointing.
#[cfg(feature = "graph")]
pub use synaptic_graph as graph;

/// Middleware system: Interceptor trait, lifecycle hooks, built-in interceptors.
#[cfg(feature = "middleware")]
pub use synaptic_middleware as middleware;

/// Event-driven system: EventBus, event subscriptions, pub-sub patterns.
#[cfg(feature = "events")]
pub use synaptic_events as events;

/// Callback handlers: Recording, Tracing, Composite, CostTracking, Metrics.
/// (Consolidated from the former synaptic-callbacks crate into synaptic-events::observers.)
#[cfg(feature = "events")]
pub use synaptic_events::callbacks;

/// Plugin system: Plugin trait, PluginManifest, PluginRegistry.
#[cfg(feature = "plugin")]
pub use synaptic_config::plugin;

/// Key-value storage: Store trait, InMemoryStore, and backend implementations.
#[cfg(feature = "store")]
pub use synaptic_store as store;

/// LLM caching: InMemory, Semantic, CachedChatModel.
#[cfg(feature = "cache")]
pub use synaptic_config::cache;

/// Evaluation: Evaluator trait, evaluators, Dataset.
#[cfg(feature = "eval")]
pub use synaptic_rag::eval;

/// MCP (Model Context Protocol) adapters for external tool servers.
#[cfg(feature = "mcp")]
pub use synaptic_mcp as mcp;

/// Procedural macros for ergonomic tool, chain, and middleware definitions.
#[cfg(feature = "macros")]
pub use synaptic_macros as macros;
/// Re-export proc macros at crate root for ergonomic use:
/// `use synaptic::tool;` instead of `use synaptic::macros::tool;`
#[cfg(feature = "macros")]
pub use synaptic_macros::*;

/// Deep agent harness: filesystem, subagents, skills, memory, auto-summarization.
#[cfg(feature = "deep")]
pub use synaptic_deep as deep;

/// Context condensation strategies: Rolling, LLM summarizing, token budget, pipeline.
#[cfg(feature = "condenser")]
pub use synaptic_middleware::condenser;

/// Secret management: SecretRegistry, SecretMaskingMiddleware.
#[cfg(feature = "secrets")]
pub use synaptic_config::secrets;

/// TOML configuration: SynapticAgentConfig, ModelConfig, McpServerConfig.
#[cfg(feature = "config")]
pub use synaptic_config as config;

/// Session lifecycle: SessionManager, SessionInfo.
#[cfg(feature = "session")]
pub use synaptic_config::session;

/// PDF document loader.
#[cfg(feature = "pdf")]
pub use synaptic_tools::pdf;

/// Tavily search tool.
#[cfg(feature = "tavily")]
pub use synaptic_integrations::tavily;

/// SQL database toolkit: ListTables, DescribeTable, ExecuteQuery (read-only).
#[cfg(feature = "sqltoolkit")]
pub use synaptic_tools::sql as sqltoolkit;

/// E2B cloud code execution sandbox.
#[cfg(feature = "e2b")]
pub use synaptic_tools::e2b;

/// Confluence wiki page loader.
#[cfg(feature = "confluence")]
pub use synaptic_integrations::confluence;

/// Slack channel message loader.
#[cfg(feature = "slack")]
pub use synaptic_integrations::slack;

/// Langfuse observability integration: LangfuseCallback, LangfuseConfig.
#[cfg(feature = "langfuse")]
pub use synaptic_integrations::langfuse;

/// Feishu/Lark integration: LarkConfig, LarkDocLoader, LarkMessageTool, LarkBitableTool.
#[cfg(feature = "lark")]
pub use synaptic_lark as lark;

/// Voice TTS/STT providers: TtsProvider, SttProvider, OpenAiVoice.
#[cfg(feature = "voice")]
pub use synaptic_integrations::voice;

/// Browser automation tools: NavigateTool, ScreenshotTool, EvalJsTool.
#[cfg(feature = "browser")]
pub use synaptic_tools::browser;

/// Job scheduling: cron + interval tasks, TokioScheduler.
#[cfg(feature = "scheduler")]
pub use synaptic_integrations::scheduler;

/// Container sandbox: Docker and Apple Container backends for secure code execution.
#[cfg(feature = "sandbox")]
pub use synaptic_tools::sandbox;

/// Structured logging: ring buffer, request-scoped spans, log query API.
#[cfg(feature = "logging")]
pub use synaptic_logging as logging;

// ---------------------------------------------------------------------------
// Provider sub-modules (consolidated into collection crates)
// ---------------------------------------------------------------------------

/// Anthropic ChatModel (consolidated into synaptic-models).
#[cfg(feature = "anthropic")]
pub mod anthropic {
    pub use synaptic_models::anthropic::*;
}

/// Google Gemini ChatModel (consolidated into synaptic-models).
#[cfg(feature = "gemini")]
pub mod gemini {
    pub use synaptic_models::gemini::*;
}

/// Ollama ChatModel and Embeddings (consolidated into synaptic-models).
#[cfg(feature = "ollama")]
pub mod ollama {
    pub use synaptic_models::ollama::*;
}

/// AWS Bedrock ChatModel (consolidated into synaptic-models).
#[cfg(feature = "bedrock")]
pub mod bedrock {
    pub use synaptic_models::bedrock::*;
}

/// Cohere Reranker (consolidated into synaptic-models).
#[cfg(feature = "cohere")]
pub mod cohere {
    pub use synaptic_models::cohere::*;
}

/// HuggingFace Inference API Embeddings (consolidated into synaptic-rag).
#[cfg(feature = "huggingface")]
pub mod huggingface {
    pub use synaptic_rag::embeddings::huggingface::*;
}

/// Voyage AI embeddings (consolidated into synaptic-rag).
#[cfg(feature = "voyage")]
pub mod voyage {
    pub use synaptic_rag::embeddings::voyage::*;
}

/// Nomic AI embeddings (consolidated into synaptic-rag).
#[cfg(feature = "nomic")]
pub mod nomic {
    pub use synaptic_rag::embeddings::nomic::*;
}

/// Jina AI embeddings and reranker (consolidated into synaptic-rag).
#[cfg(feature = "jina")]
pub mod jina {
    pub use synaptic_rag::embeddings::jina::*;
}

/// Fast local cross-encoder reranker (consolidated into synaptic-rag).
#[cfg(feature = "flashrank")]
pub mod flashrank {
    pub use synaptic_rag::embeddings::flashrank::*;
}

/// Qdrant vector store (consolidated into synaptic-rag).
#[cfg(feature = "qdrant")]
pub mod qdrant {
    pub use synaptic_rag::vectorstores::qdrant::*;
}

/// Pinecone vector store (consolidated into synaptic-rag).
#[cfg(feature = "pinecone")]
pub mod pinecone {
    pub use synaptic_rag::vectorstores::pinecone::*;
}

/// Chroma vector store (consolidated into synaptic-rag).
#[cfg(feature = "chroma")]
pub mod chroma {
    pub use synaptic_rag::vectorstores::chroma::*;
}

/// Weaviate vector store (consolidated into synaptic-rag).
#[cfg(feature = "weaviate")]
pub mod weaviate {
    pub use synaptic_rag::vectorstores::weaviate::*;
}

/// Elasticsearch vector store (consolidated into synaptic-rag).
#[cfg(feature = "elasticsearch")]
pub mod elasticsearch {
    pub use synaptic_rag::vectorstores::elasticsearch::*;
}

/// OpenSearch vector store (consolidated into synaptic-rag).
#[cfg(feature = "opensearch")]
pub mod opensearch {
    pub use synaptic_rag::vectorstores::opensearch::*;
}

/// Milvus vector store (consolidated into synaptic-rag).
#[cfg(feature = "milvus")]
pub mod milvus {
    pub use synaptic_rag::vectorstores::milvus::*;
}

/// LanceDB embedded vector store (consolidated into synaptic-rag).
#[cfg(feature = "lancedb")]
pub mod lancedb {
    pub use synaptic_rag::vectorstores::lancedb::*;
}

/// PostgreSQL integration (consolidated into synaptic-store).
#[cfg(feature = "postgres")]
pub mod postgres {
    pub use synaptic_store::postgres::*;
}

/// Redis store and cache (consolidated into synaptic-store).
#[cfg(feature = "redis")]
pub mod redis {
    pub use synaptic_store::redis::*;
}

/// SQLite integration (consolidated into synaptic-store).
#[cfg(feature = "sqlite")]
pub mod sqlite {
    pub use synaptic_store::sqlite::*;
}

/// MongoDB Atlas vector search (consolidated into synaptic-store).
#[cfg(feature = "mongodb")]
pub mod mongodb {
    pub use synaptic_store::mongodb::*;
}
