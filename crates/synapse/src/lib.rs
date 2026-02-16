//! Synapse — A Rust agent framework with LangChain-compatible architecture.
//!
//! This crate re-exports all Synapse sub-crates for convenient single-import usage.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use synapse::core::{ChatModel, Message, ChatRequest, ToolChoice};
//! use synapse::models::OpenAiChatModel;
//! use synapse::runnables::{Runnable, RunnableLambda, RunnableAssign, RunnablePick};
//! ```

/// Core traits and types: ChatModel, Message, ToolChoice, SynapseError, RunnableConfig, etc.
pub use synapse_core as core;

/// Chat model adapters: OpenAI, Anthropic, Gemini, Ollama, plus test doubles and wrappers.
pub use synapse_models as models;

/// LCEL composition: Runnable trait (with stream), BoxRunnable (with bind), pipe operator,
/// Lambda, Parallel, Branch, Assign, Pick, Fallbacks, etc.
pub use synapse_runnables as runnables;

/// Prompt templates: ChatPromptTemplate, FewShotChatMessagePromptTemplate.
pub use synapse_prompts as prompts;

/// Output parsers: Str, Json, Structured, List, Enum.
pub use synapse_parsers as parsers;

/// Tool registry and execution.
pub use synapse_tools as tools;

/// Memory strategies: Buffer, Window, Summary, SummaryBuffer, TokenBuffer, RunnableWithMessageHistory.
pub use synapse_memory as memory;

/// Callback handlers: Recording, Tracing, Composite.
pub use synapse_callbacks as callbacks;

/// Retrieval: Retriever trait, BM25, MultiQuery, Ensemble, Compression, SelfQuery, ParentDocument, Document.
pub use synapse_retrieval as retrieval;

/// Document loaders: Text, JSON, CSV, Directory.
pub use synapse_loaders as loaders;

/// Text splitters: Character, Recursive, Markdown, Token.
pub use synapse_splitters as splitters;

/// Embeddings: trait, Fake, OpenAI, Ollama.
pub use synapse_embeddings as embeddings;

/// Vector stores: InMemory, VectorStoreRetriever.
pub use synapse_vectorstores as vectorstores;

/// Graph agent orchestration: StateGraph, CompiledGraph (with stream), GraphEvent, StreamMode, checkpointing.
pub use synapse_graph as graph;

/// LLM caching: InMemory, Semantic, CachedChatModel.
pub use synapse_cache as cache;

/// Evaluation: Evaluator trait, evaluators, Dataset.
pub use synapse_eval as eval;
