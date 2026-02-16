//! Synapse — A Rust agent framework with LangChain-compatible architecture.
//!
//! This crate re-exports all Synapse sub-crates for convenient single-import usage.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use synapse::core::{ChatModel, Message, ChatRequest};
//! use synapse::models::OpenAiChatModel;
//! use synapse::runnables::{Runnable, RunnableLambda};
//! ```

/// Core traits and types: ChatModel, Message, SynapseError, RunnableConfig, etc.
pub use synapse_core as core;

/// Chat model adapters: OpenAI, Anthropic, Gemini, Ollama, plus test doubles and wrappers.
pub use synapse_models as models;

/// LCEL composition: Runnable trait, BoxRunnable, pipe operator, Lambda, Parallel, Branch, etc.
pub use synapse_runnables as runnables;

/// Sequential chain composition.
pub use synapse_chains as chains;

/// Prompt templates: ChatPromptTemplate, FewShotChatMessagePromptTemplate.
pub use synapse_prompts as prompts;

/// Output parsers: Str, Json, Structured, List, Enum.
pub use synapse_parsers as parsers;

/// Agent executors: ReActAgentExecutor.
pub use synapse_agents as agents;

/// Tool registry and execution.
pub use synapse_tools as tools;

/// Memory strategies: Buffer, Window, Summary, TokenBuffer, RunnableWithMessageHistory.
pub use synapse_memory as memory;

/// Callback handlers: Recording, Logging, Tracing, Composite.
pub use synapse_callbacks as callbacks;

/// Retrieval: Retriever trait, BM25, MultiQuery, Ensemble, Compression, Document.
pub use synapse_retrieval as retrieval;

/// Document loaders: Text, JSON, CSV, Directory.
pub use synapse_loaders as loaders;

/// Text splitters: Character, Recursive, Markdown.
pub use synapse_splitters as splitters;

/// Embeddings: trait, Fake, OpenAI, Ollama.
pub use synapse_embeddings as embeddings;

/// Vector stores: InMemory, VectorStoreRetriever.
pub use synapse_vectorstores as vectorstores;

/// Graph agent orchestration: StateGraph, CompiledGraph, checkpointing.
pub use synapse_graph as graph;

/// LLM caching: InMemory, Semantic, CachedChatModel.
pub use synapse_cache as cache;

/// Evaluation: Evaluator trait, evaluators, Dataset.
pub use synapse_eval as eval;

/// Guardrails: JSON validation.
pub use synapse_guardrails as guardrails;
