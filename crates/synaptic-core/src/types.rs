use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::chat_model::ChatResponse;
use crate::error::SynapticError;
use crate::store::{Embeddings, Store};

// ---------------------------------------------------------------------------
// RunnableConfig
// ---------------------------------------------------------------------------

/// Runtime configuration passed through runnable chains, including tags, metadata, concurrency limits, and run identification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnableConfig {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    #[serde(default)]
    pub max_concurrency: Option<usize>,
    #[serde(default)]
    pub recursion_limit: Option<usize>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub run_name: Option<String>,
}

impl RunnableConfig {
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_run_name(mut self, name: impl Into<String>) -> Self {
        self.run_name = Some(name.into());
        self
    }

    pub fn with_run_id(mut self, id: impl Into<String>) -> Self {
        self.run_id = Some(id.into());
        self
    }

    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = Some(max);
        self
    }

    pub fn with_recursion_limit(mut self, limit: usize) -> Self {
        self.recursion_limit = Some(limit);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// StreamWriter
// ---------------------------------------------------------------------------

/// Custom stream writer that nodes can use to emit custom events.
pub type StreamWriter = Arc<dyn Fn(Value) + Send + Sync>;

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Graph execution runtime context passed to nodes and middleware.
#[derive(Clone)]
pub struct Runtime {
    pub store: Option<Arc<dyn Store>>,
    pub stream_writer: Option<StreamWriter>,
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// A document with content and metadata, used throughout the retrieval pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl Document {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(
        id: impl Into<String>,
        content: impl Into<String>,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// Retriever trait
// ---------------------------------------------------------------------------

/// Trait for retrieving relevant documents given a query string.
#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(&self, query: &str, top_k: usize) -> Result<Vec<Document>, SynapticError>;
}

// ---------------------------------------------------------------------------
// VectorStore trait
// ---------------------------------------------------------------------------

/// Trait for vector storage backends.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Add documents to the store, computing their embeddings.
    async fn add_documents(
        &self,
        docs: Vec<Document>,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<String>, SynapticError>;

    /// Search for similar documents by query string.
    async fn similarity_search(
        &self,
        query: &str,
        k: usize,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<Document>, SynapticError>;

    /// Search with similarity scores (higher = more similar).
    async fn similarity_search_with_score(
        &self,
        query: &str,
        k: usize,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<(Document, f32)>, SynapticError>;

    /// Search by pre-computed embedding vector instead of text query.
    async fn similarity_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
    ) -> Result<Vec<Document>, SynapticError>;

    /// Maximum Marginal Relevance search for diverse results.
    ///
    /// `lambda_mult` controls the trade-off between relevance and diversity:
    /// - 1.0 = pure relevance (equivalent to standard similarity search)
    /// - 0.0 = maximum diversity
    ///
    /// `fetch_k` is the number of initial candidates to fetch before MMR re-ranking.
    /// Default implementation falls back to `similarity_search`.
    async fn mmr_search(
        &self,
        query: &str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<Document>, SynapticError> {
        let _ = (fetch_k, lambda_mult);
        self.similarity_search(query, k, embeddings).await
    }

    /// Delete documents by ID.
    async fn delete(&self, ids: &[&str]) -> Result<(), SynapticError>;
}

// ---------------------------------------------------------------------------
// Loader trait
// ---------------------------------------------------------------------------

/// Trait for loading documents from various sources.
#[async_trait]
pub trait Loader: Send + Sync {
    /// Load all documents from this source.
    async fn load(&self) -> Result<Vec<Document>, SynapticError>;

    /// Stream documents lazily. Default implementation wraps load().
    fn lazy_load(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<Document, SynapticError>> + Send + '_>> {
        Box::pin(async_stream::stream! {
            match self.load().await {
                Ok(docs) => {
                    for doc in docs {
                        yield Ok(doc);
                    }
                }
                Err(e) => yield Err(e),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// LlmCache trait
// ---------------------------------------------------------------------------

/// Trait for caching LLM responses.
#[async_trait]
pub trait LlmCache: Send + Sync {
    /// Look up a cached response by cache key.
    async fn get(&self, key: &str) -> Result<Option<ChatResponse>, SynapticError>;
    /// Store a response in the cache.
    async fn put(&self, key: &str, response: &ChatResponse) -> Result<(), SynapticError>;
    /// Clear all entries from the cache.
    async fn clear(&self) -> Result<(), SynapticError>;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Lifecycle events emitted during agent execution, used by `CallbackHandler` implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunEvent {
    RunStarted {
        run_id: String,
        session_id: String,
    },
    RunStep {
        run_id: String,
        step: usize,
    },
    LlmCalled {
        run_id: String,
        message_count: usize,
    },
    ToolCalled {
        run_id: String,
        tool_name: String,
    },
    RunFinished {
        run_id: String,
        output: String,
    },
    RunFailed {
        run_id: String,
        error: String,
    },
    /// Emitted before a tool call is executed.
    BeforeToolCall {
        run_id: String,
        tool_name: String,
        arguments: String,
    },
    /// Emitted after a tool call completes.
    AfterToolCall {
        run_id: String,
        tool_name: String,
        result: String,
    },
    /// Emitted before a message is sent to the model.
    BeforeMessage {
        run_id: String,
        message_count: usize,
    },
    /// Emitted after a model response is received.
    AfterMessage {
        run_id: String,
        response_length: usize,
    },
}

// ---------------------------------------------------------------------------
// CallbackHandler
// ---------------------------------------------------------------------------

/// Handler for lifecycle events during agent execution. Receives `RunEvent` notifications at each stage.
#[async_trait]
pub trait CallbackHandler: Send + Sync {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapticError>;
}

// ---------------------------------------------------------------------------
// Entrypoint / Task metadata (used by proc macros)
// ---------------------------------------------------------------------------

/// Configuration for an `#[entrypoint]`-decorated function.
#[derive(Debug, Clone)]
pub struct EntrypointConfig {
    pub name: &'static str,
    pub checkpointer: Option<&'static str>,
}

/// An entrypoint wrapping an async function as a runnable workflow.
///
/// The `invoke_fn` field is a type-erased async function (`Value -> Result<Value, SynapticError>`).
/// Type alias for the async entrypoint function signature.
pub type EntrypointFn = dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, SynapticError>> + Send>>
    + Send
    + Sync;

pub struct Entrypoint {
    pub config: EntrypointConfig,
    pub invoke_fn: Box<EntrypointFn>,
}

impl Entrypoint {
    pub async fn invoke(&self, input: Value) -> Result<Value, SynapticError> {
        (self.invoke_fn)(input).await
    }
}
