use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;

/// A single memory search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResult {
    /// Unique resource identifier for this memory entry.
    pub uri: String,
    /// The text content of the memory entry.
    pub content: String,
    /// Relevance score (higher is more relevant).
    pub score: f64,
    /// Optional semantic category (e.g. "fact", "preference", "skill").
    pub category: Option<String>,
    /// Memory layer this result comes from (e.g. "working", "episodic", "semantic").
    pub layer: Option<String>,
    /// Arbitrary provider-specific metadata.
    pub metadata: serde_json::Value,
}

/// Result of committing (archiving) a session into long-term memory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommitResult {
    /// Whether the session transcript was archived.
    pub archived: bool,
    /// Number of new memory entries extracted from the session.
    pub memories_extracted: usize,
    /// Number of existing memory entries updated/merged.
    pub memories_merged: usize,
    /// Number of candidate entries skipped (duplicates, low-quality, etc.).
    pub memories_skipped: usize,
}

/// Abstraction over a long-term memory backend.
///
/// Implementors may store memories in a vector database, an SQL store, an
/// in-process index, or any combination thereof.  All methods are `async` and
/// the trait is object-safe via `async_trait`.
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// Append a single conversation turn to the session buffer.
    ///
    /// `role` is typically `"user"` or `"assistant"`.
    async fn add_message(
        &self,
        session_key: &str,
        role: &str,
        content: &str,
    ) -> Result<(), SynapticError>;

    /// Record that the session consumed particular context or skill URIs so that
    /// the memory backend can update usage statistics / weights.
    async fn record_usage(
        &self,
        session_key: &str,
        context_uris: &[String],
        skill_uris: &[String],
    ) -> Result<(), SynapticError>;

    /// Retrieve the most relevant memories for `query` across all sessions.
    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>, SynapticError>;

    /// Search memories, optionally scoped to a specific session.
    async fn search(
        &self,
        query: &str,
        session_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, SynapticError>;

    /// Commit the session buffer to long-term storage, extracting and merging
    /// memories.  Returns a summary of what was written.
    async fn commit(&self, session_key: &str) -> Result<CommitResult, SynapticError>;

    /// Index an external resource (identified by `uri`) so it can be recalled
    /// in future searches.
    async fn add_resource(&self, uri: &str) -> Result<(), SynapticError>;

    /// Retrieve a human-readable profile summary for the given user, or `None`
    /// if no profile has been built yet.
    async fn get_profile(&self, user_id: &str) -> Result<Option<String>, SynapticError>;
}
