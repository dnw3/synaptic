use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::SynapticError;
use crate::message::Message;

// ---------------------------------------------------------------------------
// Item
// ---------------------------------------------------------------------------

/// A stored item in the key-value store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub namespace: Vec<String>,
    pub key: String,
    pub value: Value,
    pub created_at: String,
    pub updated_at: String,
    /// Relevance score from a search operation (e.g., similarity score).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

// ---------------------------------------------------------------------------
// SearchOptions
// ---------------------------------------------------------------------------

/// Options for advanced store search with temporal decay and score filtering.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Text query (same as the query parameter in `Store::search`).
    pub query: Option<String>,
    /// Maximum number of results to return.
    pub limit: usize,
    /// Temporal decay half-life in seconds. Scores are multiplied by
    /// `exp(-ln2 / half_life * age_secs)` so older items decay exponentially.
    pub decay_half_life_secs: Option<u64>,
    /// Minimum score threshold (after decay). Items below are excluded.
    pub min_score: Option<f64>,
}

impl SearchOptions {
    pub fn new(limit: usize) -> Self {
        Self {
            query: None,
            limit,
            decay_half_life_secs: None,
            min_score: None,
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_decay(mut self, half_life_secs: u64) -> Self {
        self.decay_half_life_secs = Some(half_life_secs);
        self
    }

    pub fn with_min_score(mut self, min_score: f64) -> Self {
        self.min_score = Some(min_score);
        self
    }
}

// ---------------------------------------------------------------------------
// Store trait
// ---------------------------------------------------------------------------

/// Persistent key-value store trait for cross-invocation state.
///
/// Namespaces are hierarchical (represented as slices of strings) and
/// keys are strings within a namespace. Values are arbitrary JSON.
#[async_trait]
pub trait Store: Send + Sync {
    /// Get an item by namespace and key.
    async fn get(&self, namespace: &[&str], key: &str) -> Result<Option<Item>, SynapticError>;

    /// Search items within a namespace.
    async fn search(
        &self,
        namespace: &[&str],
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Item>, SynapticError>;

    /// Advanced search with temporal decay and score filtering.
    ///
    /// Default implementation delegates to `search()` and applies decay post-hoc.
    async fn search_with_options(
        &self,
        namespace: &[&str],
        options: &SearchOptions,
    ) -> Result<Vec<Item>, SynapticError> {
        let mut items = self
            .search(namespace, options.query.as_deref(), options.limit * 2)
            .await?;

        if let Some(half_life) = options.decay_half_life_secs {
            let now = chrono::Utc::now();
            let lambda = std::f64::consts::LN_2 / half_life as f64;
            for item in &mut items {
                let age_secs = parse_age_secs(&item.created_at, now);
                let decay = (-lambda * age_secs).exp();
                let base_score = item.score.unwrap_or(1.0);
                item.score = Some(base_score * decay);
            }
            items.sort_by(|a, b| {
                b.score
                    .unwrap_or(0.0)
                    .partial_cmp(&a.score.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        if let Some(min) = options.min_score {
            items.retain(|item| item.score.unwrap_or(0.0) >= min);
        }

        items.truncate(options.limit);
        Ok(items)
    }

    /// Put (upsert) an item.
    async fn put(&self, namespace: &[&str], key: &str, value: Value) -> Result<(), SynapticError>;

    /// Delete an item.
    async fn delete(&self, namespace: &[&str], key: &str) -> Result<(), SynapticError>;

    /// List all namespaces, optionally filtered by prefix.
    async fn list_namespaces(&self, prefix: &[&str]) -> Result<Vec<Vec<String>>, SynapticError>;
}

/// Parse an ISO 8601 timestamp and return the age in seconds relative to `now`.
pub fn parse_age_secs(timestamp: &str, now: chrono::DateTime<chrono::Utc>) -> f64 {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|dt| dt.and_utc().fixed_offset())
        })
        .map(|dt| (now - dt.with_timezone(&chrono::Utc)).num_seconds().max(0) as f64)
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// MemoryStore
// ---------------------------------------------------------------------------

/// Persistent storage for conversation message history, keyed by session ID.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn append(&self, session_id: &str, message: Message) -> Result<(), SynapticError>;
    async fn load(&self, session_id: &str) -> Result<Vec<Message>, SynapticError>;
    async fn clear(&self, session_id: &str) -> Result<(), SynapticError>;
}

// ---------------------------------------------------------------------------
// Embeddings trait
// ---------------------------------------------------------------------------

/// Trait for embedding text into vectors.
#[async_trait]
pub trait Embeddings: Send + Sync {
    /// Embed multiple texts (for batch document embedding).
    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SynapticError>;

    /// Embed a single query text.
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, SynapticError>;
}

// ---------------------------------------------------------------------------
// Shared store utilities
// ---------------------------------------------------------------------------

/// ISO 8601 timestamp for store metadata.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Encode a namespace slice as a colon-separated string.
pub fn encode_namespace(namespace: &[&str]) -> String {
    namespace.join(":")
}

/// Validate a SQL table name (alphanumeric, underscore, and dot only).
pub fn validate_table_name(name: &str) -> Result<(), SynapticError> {
    if name.is_empty() {
        return Err(SynapticError::Store(
            "table name must not be empty".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return Err(SynapticError::Store(format!(
            "invalid table name '{name}': only alphanumeric, underscore, and dot characters are allowed",
        )));
    }
    Ok(())
}
