use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Unified error type for the Synaptic framework with variants covering all subsystems.
#[derive(Debug, Error)]
pub enum SynapticError {
    #[error("prompt error: {0}")]
    Prompt(String),
    #[error("model error: {0}")]
    Model(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("memory error: {0}")]
    Memory(String),
    #[error("rate limit: {0}")]
    RateLimit(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("parsing error: {0}")]
    Parsing(String),
    #[error("callback error: {0}")]
    Callback(String),
    #[error("max steps exceeded: {max_steps}")]
    MaxStepsExceeded { max_steps: usize },
    #[error("embedding error: {0}")]
    Embedding(String),
    #[error("vector store error: {0}")]
    VectorStore(String),
    #[error("retriever error: {0}")]
    Retriever(String),
    #[error("loader error: {0}")]
    Loader(String),
    #[error("splitter error: {0}")]
    Splitter(String),
    #[error("graph error: {0}")]
    Graph(String),
    #[error("cache error: {0}")]
    Cache(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("mcp error: {0}")]
    Mcp(String),
    #[error("security error: {0}")]
    Security(String),
}
