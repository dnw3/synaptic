//! Synaptic — A Rust agent framework with LangChain-compatible architecture.
//!
//! This crate re-exports all Synaptic sub-crates for convenient single-import usage.
//! Enable features to control which modules are available.
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `default` | `runnables`, `prompts`, `parsers`, `tools`, `callbacks` |
//! | `model-utils` | `ProviderBackend`, `ScriptedChatModel`, wrappers (Retry, RateLimit, etc.) |
//! | `openai` | OpenAI ChatModel + Embeddings |
//! | `anthropic` | Anthropic ChatModel |
//! | `gemini` | Gemini ChatModel |
//! | `ollama` | Ollama ChatModel + Embeddings |
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
//! use synaptic::runnables::{Runnable, RunnableLambda, RunnableAssign, RunnablePick};
//! ```

// Re-export internal crates under their original names so proc-macro generated code
// (which references `::synaptic::synaptic_core`, etc. via `proc-macro-crate` detection)
// can resolve correctly when downstream crates only depend on the `synaptic` facade.
#[doc(hidden)]
pub extern crate synaptic_core;
#[cfg(feature = "middleware")]
#[doc(hidden)]
pub extern crate synaptic_middleware;
#[cfg(feature = "runnables")]
#[doc(hidden)]
pub extern crate synaptic_runnables;

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

/// ProviderBackend abstraction, ScriptedChatModel, and ChatModel wrappers
/// (Retry, RateLimit, TokenBucket, StructuredOutput, BoundTools).
#[cfg(feature = "model-utils")]
pub use synaptic_models as models;

/// OpenAI ChatModel and Embeddings.
#[cfg(feature = "openai")]
pub use synaptic_openai as openai;

/// Anthropic ChatModel.
#[cfg(feature = "anthropic")]
pub use synaptic_anthropic as anthropic;

/// Google Gemini ChatModel.
#[cfg(feature = "gemini")]
pub use synaptic_gemini as gemini;

/// Ollama ChatModel and Embeddings.
#[cfg(feature = "ollama")]
pub use synaptic_ollama as ollama;

/// LCEL composition: Runnable trait (with stream), BoxRunnable (with bind), pipe operator,
/// Lambda, Parallel, Branch, Assign, Pick, Fallbacks, etc.
#[cfg(feature = "runnables")]
pub use synaptic_runnables as runnables;

/// Prompt templates: ChatPromptTemplate, FewShotChatMessagePromptTemplate.
#[cfg(feature = "prompts")]
pub use synaptic_prompts as prompts;

/// Output parsers: Str, Json, Structured, List, Enum.
#[cfg(feature = "parsers")]
pub use synaptic_parsers as parsers;

/// Tool registry and execution.
#[cfg(feature = "tools")]
pub use synaptic_tools as tools;

/// Memory strategies: Buffer, Window, Summary, SummaryBuffer, TokenBuffer, RunnableWithMessageHistory.
#[cfg(feature = "memory")]
pub use synaptic_memory as memory;

/// Callback handlers: Recording, Tracing, Composite.
#[cfg(feature = "callbacks")]
pub use synaptic_callbacks as callbacks;

/// Retrieval: Retriever trait, BM25, MultiQuery, Ensemble, Compression, SelfQuery, ParentDocument, Document.
#[cfg(feature = "retrieval")]
pub use synaptic_retrieval as retrieval;

/// Document loaders: Text, JSON, CSV, Directory.
#[cfg(feature = "loaders")]
pub use synaptic_loaders as loaders;

/// Text splitters: Character, Recursive, Markdown, Token.
#[cfg(feature = "splitters")]
pub use synaptic_splitters as splitters;

/// Embeddings: trait, Fake, CacheBacked.
#[cfg(feature = "embeddings")]
pub use synaptic_embeddings as embeddings;

/// Vector stores: InMemory, VectorStoreRetriever.
#[cfg(feature = "vectorstores")]
pub use synaptic_vectorstores as vectorstores;

/// Graph agent orchestration: StateGraph, CompiledGraph (with stream), GraphEvent, StreamMode, checkpointing.
#[cfg(feature = "graph")]
pub use synaptic_graph as graph;

/// Middleware system: AgentMiddleware trait, lifecycle hooks, built-in middlewares.
#[cfg(feature = "middleware")]
pub use synaptic_middleware as middleware;

/// Event-driven system: EventBus, event subscriptions, pub-sub patterns.
#[cfg(feature = "events")]
pub use synaptic_events as events;

/// Plugin system: Plugin trait, PluginManifest, PluginRegistry.
#[cfg(feature = "plugin")]
pub use synaptic_plugin as plugin;

/// Key-value storage: Store trait, InMemoryStore.
#[cfg(feature = "store")]
pub use synaptic_store as store;

/// LLM caching: InMemory, Semantic, CachedChatModel.
#[cfg(feature = "cache")]
pub use synaptic_cache as cache;

/// Evaluation: Evaluator trait, evaluators, Dataset.
#[cfg(feature = "eval")]
pub use synaptic_eval as eval;

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
pub use synaptic_condenser as condenser;

/// Secret management: SecretRegistry, SecretMaskingMiddleware.
#[cfg(feature = "secrets")]
pub use synaptic_secrets as secrets;

/// TOML configuration: SynapticAgentConfig, ModelConfig, McpServerConfig.
#[cfg(feature = "config")]
pub use synaptic_config as config;

/// Session lifecycle: SessionManager, SessionInfo.
#[cfg(feature = "session")]
pub use synaptic_session as session;

/// Qdrant vector store integration.
#[cfg(feature = "qdrant")]
pub use synaptic_qdrant as qdrant;

/// PostgreSQL integration.
#[cfg(feature = "postgres")]
pub use synaptic_postgres as postgres;

/// Redis store and cache integration.
#[cfg(feature = "redis")]
pub use synaptic_redis as redis;

/// PDF document loader.
#[cfg(feature = "pdf")]
pub use synaptic_pdf as pdf;

/// AWS Bedrock ChatModel.
#[cfg(feature = "bedrock")]
pub use synaptic_bedrock as bedrock;

/// Cohere Reranker.
#[cfg(feature = "cohere")]
pub use synaptic_cohere as cohere;

/// Pinecone vector store.
#[cfg(feature = "pinecone")]
pub use synaptic_pinecone as pinecone;

/// Chroma vector store.
#[cfg(feature = "chroma")]
pub use synaptic_chroma as chroma;

/// MongoDB Atlas vector search.
#[cfg(feature = "mongodb")]
pub use synaptic_mongodb as mongodb;

/// Elasticsearch vector store.
#[cfg(feature = "elasticsearch")]
pub use synaptic_elasticsearch as elasticsearch;

/// SQLite integration.
#[cfg(feature = "sqlite")]
pub use synaptic_sqlite as sqlite;

/// Tavily search tool.
#[cfg(feature = "tavily")]
pub use synaptic_tavily as tavily;

/// HuggingFace Inference API Embeddings.
#[cfg(feature = "huggingface")]
pub use synaptic_huggingface as huggingface;

/// Voyage AI embeddings (voyage-3-large, voyage-code-3, etc.).
#[cfg(feature = "voyage")]
pub use synaptic_voyage as voyage;

/// Nomic AI embeddings (nomic-embed-text-v1.5, open weights).
#[cfg(feature = "nomic")]
pub use synaptic_nomic as nomic;

/// Jina AI embeddings and reranker.
#[cfg(feature = "jina")]
pub use synaptic_jina as jina;

/// Weaviate vector database integration.
#[cfg(feature = "weaviate")]
pub use synaptic_weaviate as weaviate;

/// SQL database toolkit: ListTables, DescribeTable, ExecuteQuery (read-only).
#[cfg(feature = "sqltoolkit")]
pub use synaptic_sqltoolkit as sqltoolkit;

/// E2B cloud code execution sandbox.
#[cfg(feature = "e2b")]
pub use synaptic_e2b as e2b;

/// Milvus vector store.
#[cfg(feature = "milvus")]
pub use synaptic_milvus as milvus;

/// OpenSearch vector store.
#[cfg(feature = "opensearch")]
pub use synaptic_opensearch as opensearch;

/// LanceDB embedded vector store.
#[cfg(feature = "lancedb")]
pub use synaptic_lancedb as lancedb;

/// Confluence wiki page loader.
#[cfg(feature = "confluence")]
pub use synaptic_confluence as confluence;

/// Slack channel message loader.
#[cfg(feature = "slack")]
pub use synaptic_slack as slack;

/// Fast local cross-encoder reranker (BM25-based, zero external dependencies).
#[cfg(feature = "flashrank")]
pub use synaptic_flashrank as flashrank;

/// Langfuse observability integration: LangfuseCallback, LangfuseConfig.
#[cfg(feature = "langfuse")]
pub use synaptic_langfuse as langfuse;

/// Feishu/Lark integration: LarkConfig, LarkDocLoader, LarkMessageTool, LarkBitableTool.
#[cfg(feature = "lark")]
pub use synaptic_lark as lark;

/// Voice TTS/STT providers: TtsProvider, SttProvider, OpenAiVoice.
#[cfg(feature = "voice")]
pub use synaptic_voice as voice;

/// Browser automation tools: NavigateTool, ScreenshotTool, EvalJsTool.
#[cfg(feature = "browser")]
pub use synaptic_browser as browser;

/// Job scheduling: cron + interval tasks, TokioScheduler.
#[cfg(feature = "scheduler")]
pub use synaptic_scheduler as scheduler;

/// Container sandbox: Docker and Apple Container backends for secure code execution.
#[cfg(feature = "sandbox")]
pub use synaptic_sandbox as sandbox;

/// Prometheus metrics exporter: render and serve /metrics endpoint.
#[cfg(feature = "metrics")]
pub use synaptic_metrics as metrics;
