use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use synaptic_core::Message;
use tokio::sync::RwLock;

/// Holds the result of a background sub-agent execution.
#[derive(Debug, Clone)]
pub struct BackgroundTaskResult {
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_secs: Option<f64>,
    /// User-supplied label for this task.
    pub label: Option<String>,
    /// Conversation history preserved for resume support.
    pub messages: Option<Vec<Message>>,
    /// When the task completed (for auto-cleanup).
    pub completed_at: Option<std::time::Instant>,
}

/// Registry for tracking background sub-agent tasks.
#[derive(Default)]
pub struct BackgroundTaskRegistry {
    next_id: AtomicU64,
    tasks: RwLock<HashMap<String, BackgroundTaskResult>>,
    abort_handles: RwLock<HashMap<String, tokio::task::AbortHandle>>,
    /// Tracks running children per agent type for maxChildrenPerAgent.
    active_children: RwLock<HashMap<String, usize>>,
    /// Auto-remove completed tasks after this many seconds (0 = never).
    archive_after_secs: AtomicU64,
}

impl BackgroundTaskRegistry {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            tasks: RwLock::new(HashMap::new()),
            abort_handles: RwLock::new(HashMap::new()),
            active_children: RwLock::new(HashMap::new()),
            archive_after_secs: AtomicU64::new(0),
        }
    }

    /// Set auto-cleanup duration. Completed tasks are removed after this many seconds.
    /// Set to 0 to disable (default).
    pub fn set_archive_after_secs(&self, secs: u64) {
        self.archive_after_secs.store(secs, Ordering::Relaxed);
    }

    /// Remove completed/failed tasks that have exceeded the archive timeout.
    async fn cleanup_archived(&self) {
        let secs = self.archive_after_secs.load(Ordering::Relaxed);
        if secs == 0 {
            return;
        }
        let cutoff = std::time::Duration::from_secs(secs);
        let now = std::time::Instant::now();
        let mut tasks = self.tasks.write().await;
        tasks.retain(|_, t| {
            if let Some(completed_at) = t.completed_at {
                now.duration_since(completed_at) < cutoff
            } else {
                true // keep running tasks
            }
        });
    }

    pub(crate) fn allocate_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("agent-{}", id)
    }

    pub(crate) async fn set_running(&self, id: &str, label: Option<String>) {
        // Opportunistically clean up old completed tasks
        self.cleanup_archived().await;

        let mut tasks = self.tasks.write().await;
        tasks.insert(
            id.to_string(),
            BackgroundTaskResult {
                status: "running".to_string(),
                result: None,
                error: None,
                duration_secs: None,
                label,
                messages: None,
                completed_at: None,
            },
        );
    }

    pub(crate) async fn set_completed(
        &self,
        id: &str,
        result: String,
        duration_secs: f64,
        messages: Option<Vec<Message>>,
    ) {
        let mut tasks = self.tasks.write().await;
        let label = tasks.get(id).and_then(|t| t.label.clone());
        tasks.insert(
            id.to_string(),
            BackgroundTaskResult {
                status: "completed".to_string(),
                result: Some(result),
                error: None,
                duration_secs: Some(duration_secs),
                label,
                messages,
                completed_at: Some(std::time::Instant::now()),
            },
        );
    }

    pub(crate) async fn set_failed(&self, id: &str, error: String) {
        let mut tasks = self.tasks.write().await;
        let label = tasks.get(id).and_then(|t| t.label.clone());
        tasks.insert(
            id.to_string(),
            BackgroundTaskResult {
                status: "failed".to_string(),
                result: None,
                error: Some(error),
                duration_secs: None,
                label,
                messages: None,
                completed_at: Some(std::time::Instant::now()),
            },
        );
    }

    /// Register an abort handle for a background task.
    pub(crate) async fn register_abort_handle(&self, id: &str, handle: tokio::task::AbortHandle) {
        self.abort_handles
            .write()
            .await
            .insert(id.to_string(), handle);
    }

    /// Kill a running background task. Returns true if it was aborted.
    pub async fn kill(&self, id: &str) -> bool {
        if let Some(handle) = self.abort_handles.write().await.remove(id) {
            handle.abort();
            self.set_failed(id, "killed by user".to_string()).await;
            true
        } else {
            false
        }
    }

    /// Get the current status of a background task.
    pub async fn get(&self, id: &str) -> Option<BackgroundTaskResult> {
        self.tasks.read().await.get(id).cloned()
    }

    /// Increment active children count for an agent type.
    pub async fn increment_children(&self, agent_type: &str) {
        let mut map = self.active_children.write().await;
        *map.entry(agent_type.to_string()).or_insert(0) += 1;
    }

    /// Decrement active children count for an agent type.
    pub async fn decrement_children(&self, agent_type: &str) {
        let mut map = self.active_children.write().await;
        if let Some(count) = map.get_mut(agent_type) {
            *count = count.saturating_sub(1);
        }
    }

    /// Get the current active children count for an agent type.
    pub async fn active_children_count(&self, agent_type: &str) -> usize {
        self.active_children
            .read()
            .await
            .get(agent_type)
            .copied()
            .unwrap_or(0)
    }
}
