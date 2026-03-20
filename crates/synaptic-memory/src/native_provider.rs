use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::SynapticError;

use crate::provider::{CommitResult, MemoryProvider, MemoryResult};

/// A function that performs a recall query and returns matching content strings.
///
/// The function takes `(query, limit)` and returns a future resolving to a
/// `Vec<String>` of memory contents.
pub type RecallFn =
    Arc<dyn Fn(String, usize) -> Pin<Box<dyn Future<Output = Vec<String>> + Send>> + Send + Sync>;

/// A [`MemoryProvider`] implementation backed by a pluggable recall function.
///
/// This is designed for providers that wrap an existing local memory store
/// (e.g. an embedding-based LTM). The caller injects a `RecallFn` callback at
/// construction time; all other `MemoryProvider` methods are no-ops.
pub struct NativeMemoryProvider {
    recall_fn: Option<RecallFn>,
}

impl NativeMemoryProvider {
    /// Create a provider that delegates `recall` to the given callback.
    pub fn new(recall_fn: RecallFn) -> Self {
        Self {
            recall_fn: Some(recall_fn),
        }
    }

    /// Create a no-op provider that returns empty results for all queries.
    ///
    /// Used when no backing memory store is available.
    pub fn new_noop() -> Self {
        Self { recall_fn: None }
    }
}

#[async_trait]
impl MemoryProvider for NativeMemoryProvider {
    async fn add_message(
        &self,
        _session_key: &str,
        _role: &str,
        _content: &str,
    ) -> Result<(), SynapticError> {
        Ok(())
    }

    async fn record_usage(
        &self,
        _session_key: &str,
        _context_uris: &[String],
        _skill_uris: &[String],
    ) -> Result<(), SynapticError> {
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>, SynapticError> {
        let Some(ref recall_fn) = self.recall_fn else {
            return Ok(Vec::new());
        };
        let contents = (recall_fn)(query.to_string(), limit).await;
        let results = contents
            .into_iter()
            .enumerate()
            .map(|(i, content)| MemoryResult {
                uri: format!("ltm:{}", i),
                content,
                score: 1.0,
                category: None,
                layer: Some("semantic".to_string()),
                metadata: serde_json::Value::Null,
            })
            .collect();
        Ok(results)
    }

    async fn search(
        &self,
        query: &str,
        _session_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, SynapticError> {
        self.recall(query, limit).await
    }

    async fn commit(&self, _session_key: &str) -> Result<CommitResult, SynapticError> {
        Ok(CommitResult::default())
    }

    async fn add_resource(&self, _uri: &str) -> Result<(), SynapticError> {
        Err(SynapticError::Tool(
            "NativeMemoryProvider does not support resource ingestion".into(),
        ))
    }

    async fn get_profile(&self, _user_id: &str) -> Result<Option<String>, SynapticError> {
        Ok(None)
    }
}
