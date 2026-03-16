use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use synaptic_core::{now_iso, Store, SynapticError};
use synaptic_graph::StoreCheckpointer;
use synaptic_memory::ChatMessageHistory;

/// Returns the current time as Unix milliseconds.
fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Metadata about a session.
///
/// This struct has been expanded to 25+ fields for OpenClaw alignment.
/// Backward compatibility: the old `"id"` and `"token_count"` JSON keys
/// are accepted via serde aliases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    // === Identity ===
    /// Unique session identifier (alias: `id` for backward compat).
    #[serde(alias = "id")]
    pub session_id: String,
    /// Optional application-level key (e.g. `"agent:default:main"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Last-updated timestamp in Unix milliseconds.
    #[serde(default)]
    pub updated_at: u64,

    // === Channel Origin ===
    /// Originating channel (e.g. `"lark"`, `"slack"`, `"web"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Chat type (e.g. `"p2p"`, `"group"`, `"topic"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_type: Option<String>,
    /// Human-readable display name for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// User-assigned label / tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    // === Token Tracking ===
    /// Cumulative input tokens.
    #[serde(default)]
    pub input_tokens: u64,
    /// Cumulative output tokens.
    #[serde(default)]
    pub output_tokens: u64,
    /// Cumulative total tokens (alias: `token_count` for backward compat).
    #[serde(default, alias = "token_count")]
    pub total_tokens: u64,
    /// Whether `total_tokens` reflects actual API usage (true) or a heuristic estimate (false).
    #[serde(default)]
    pub total_tokens_fresh: bool,
    /// Number of times this session was compacted.
    #[serde(default)]
    pub compaction_count: u32,

    // === Per-Session Model Override ===
    /// Model name override for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider override for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,

    // === Per-Session Runtime Config ===
    /// Thinking / extended-thinking level override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
    /// Verbose level override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbose_level: Option<String>,
    /// Fast mode toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast_mode: Option<bool>,
    /// Reasoning level override (e.g. `"low"`, `"medium"`, `"high"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_level: Option<String>,

    // === Execution State ===
    /// Whether the system prompt has been sent for this session.
    #[serde(default)]
    pub system_sent: bool,
    /// Whether the last agent run was aborted.
    #[serde(default)]
    pub aborted_last_run: bool,
    /// Send policy override (e.g. `"always"`, `"on-change"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_policy: Option<String>,

    // === Delivery Routing ===
    /// Last channel through which a reply was delivered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_channel: Option<String>,
    /// Last delivery target (e.g. chat ID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_to: Option<String>,
    /// Last account / bot ID used for delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_account_id: Option<String>,
    /// Last thread ID for threaded replies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_thread_id: Option<String>,

    // === Spawned Session (Subagent) ===
    /// Session ID of the parent that spawned this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawned_by: Option<String>,
    /// Nesting depth for subagent spawns.
    #[serde(default)]
    pub spawn_depth: u32,

    // === Group / Thread ===
    /// Group / conversation ID that this session belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// Subject / title for group conversations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Whether this session was forked from a parent session.
    #[serde(default)]
    pub forked_from_parent: bool,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            session_key: None,
            created_at: String::new(),
            updated_at: 0,
            channel: None,
            chat_type: None,
            display_name: None,
            label: None,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            total_tokens_fresh: false,
            compaction_count: 0,
            model: None,
            model_provider: None,
            thinking_level: None,
            verbose_level: None,
            fast_mode: None,
            reasoning_level: None,
            system_sent: false,
            aborted_last_run: false,
            send_policy: None,
            last_channel: None,
            last_to: None,
            last_account_id: None,
            last_thread_id: None,
            spawned_by: None,
            spawn_depth: 0,
            group_id: None,
            subject: None,
            forked_from_parent: false,
        }
    }
}

/// Store-backed session manager.
///
/// Session metadata is stored under namespace `["sessions"]`, key = session_id.
/// Messages are accessed through [`ChatMessageHistory`] (same store).
/// Checkpoints are accessed through [`StoreCheckpointer`] (same store).
pub struct SessionManager {
    store: Arc<dyn Store>,
}

impl SessionManager {
    /// Create a new session manager backed by the given store.
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }

    /// Create a new session with a unique ID.
    pub async fn create_session(&self) -> Result<String, SynapticError> {
        let id = uuid::Uuid::new_v4().to_string();
        let info = SessionInfo {
            session_id: id.clone(),
            created_at: now_iso(),
            updated_at: now_unix_ms(),
            ..Default::default()
        };
        let value = serde_json::to_value(&info)
            .map_err(|e| SynapticError::Store(format!("failed to serialize session info: {e}")))?;
        self.store.put(&["sessions"], &id, value).await?;
        Ok(id)
    }

    /// List all sessions.
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>, SynapticError> {
        let items = self.store.search(&["sessions"], None, 10_000).await?;
        let mut sessions: Vec<SessionInfo> = items
            .into_iter()
            .filter_map(|item| serde_json::from_value(item.value).ok())
            .collect();
        sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(sessions)
    }

    /// Get session info by ID.
    pub async fn get_session(&self, id: &str) -> Result<Option<SessionInfo>, SynapticError> {
        let item = self.store.get(&["sessions"], id).await?;
        match item {
            Some(item) => {
                let info: SessionInfo = serde_json::from_value(item.value).map_err(|e| {
                    SynapticError::Store(format!("failed to deserialize session info: {e}"))
                })?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    /// Update session metadata (e.g. total_tokens, compaction_count).
    pub async fn update_session(&self, info: &SessionInfo) -> Result<(), SynapticError> {
        let value = serde_json::to_value(info)
            .map_err(|e| SynapticError::Store(format!("failed to serialize session info: {e}")))?;
        self.store
            .put(&["sessions"], &info.session_id, value)
            .await?;
        Ok(())
    }

    /// Delete a session and all its associated data (messages, summaries, checkpoints).
    pub async fn delete_session(&self, id: &str) -> Result<(), SynapticError> {
        // Delete session metadata
        self.store.delete(&["sessions"], id).await?;

        // Delete messages and summary
        self.store.delete(&["memory", id], "messages").await?;
        self.store.delete(&["memory", id], "summary").await?;

        // Delete checkpoints — search and delete each one
        let checkpoints = self
            .store
            .search(&["checkpoints", id], None, 10_000)
            .await?;
        for ckpt in checkpoints {
            self.store.delete(&["checkpoints", id], &ckpt.key).await?;
        }

        Ok(())
    }

    /// Get a `ChatMessageHistory` that shares the same store.
    pub fn memory(&self) -> ChatMessageHistory {
        ChatMessageHistory::new(self.store.clone())
    }

    /// Get a `StoreCheckpointer` that shares the same store.
    pub fn checkpointer(&self) -> StoreCheckpointer {
        StoreCheckpointer::new(self.store.clone())
    }

    /// Get a reference to the underlying store.
    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }
}
