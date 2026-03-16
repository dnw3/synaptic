use std::pin::Pin;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Shared message format between channels and the agent system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub channel_id: String,
    pub sender_id: String,
    pub content: String,
    pub thread_id: Option<String>,
    pub attachments: Vec<Attachment>,
    pub metadata: Value,
}

/// A file or media attachment carried by a [`MessageEnvelope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub url: String,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
}

/// Static description of a channel adapter's identity and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelManifest {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<ChannelCap>,
    pub message_limit: Option<usize>,
    pub supports_streaming: bool,
    pub supports_threads: bool,
    pub supports_reactions: bool,
}

/// Fine-grained capability flags declared by a channel adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelCap {
    Inbound,
    Outbound,
    Auth,
    Threading,
    Groups,
    Reactions,
    Mentions,
    PlatformActions,
    Config,
    Health,
}

/// Runtime connection state of a channel adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

/// Outcome of a health check.
#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

/// Policy governing which groups the adapter will serve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupPolicy {
    AllowAll,
    Allowlist,
    DenyAll,
}

/// Lightweight descriptor for a group/channel on the remote platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
}

/// A parsed @mention extracted from a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub user_id: String,
    pub display_name: Option<String>,
}

/// A platform-specific action that the adapter can execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub name: String,
    pub description: String,
    pub parameters: Option<Value>,
}

/// Runtime context passed to an adapter when it is started.
pub struct ChannelContext {
    pub config: Value,
}

// ---------------------------------------------------------------------------
// Required base trait
// ---------------------------------------------------------------------------

/// Required base trait for all channel adapters.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Return the adapter's static manifest.
    fn manifest(&self) -> ChannelManifest;

    /// Start the adapter with the supplied runtime context.
    async fn start(&self, ctx: ChannelContext) -> Result<(), crate::SynapticError>;

    /// Gracefully stop the adapter.
    async fn stop(&self) -> Result<(), crate::SynapticError>;

    /// Return the current connection status.
    fn status(&self) -> ChannelStatus;
}

// ---------------------------------------------------------------------------
// Optional capability traits
// ---------------------------------------------------------------------------

/// Inbound messages — stream-based.
///
/// Adapters typically spawn a background task that feeds incoming messages into
/// an internal `mpsc` channel; this method returns a `Stream` over that channel.
pub trait Inbound: ChannelAdapter {
    fn message_stream(&self) -> Pin<Box<dyn futures::Stream<Item = MessageEnvelope> + Send>>;
}

/// Outbound message sending.
#[async_trait]
pub trait Outbound: ChannelAdapter {
    /// Send a message to the channel described by `envelope`.
    async fn send(&self, envelope: &MessageEnvelope) -> Result<(), crate::SynapticError>;

    /// Edit a previously sent message (default: unsupported).
    async fn edit(&self, _msg_id: &str, _content: &str) -> Result<(), crate::SynapticError> {
        Err(crate::SynapticError::Tool("edit not supported".into()))
    }
}

/// Channel authentication.
#[async_trait]
pub trait ChannelAuth: ChannelAdapter {
    /// Authenticate using the supplied configuration.
    async fn login(&self, config: &Value) -> Result<(), crate::SynapticError>;

    /// Invalidate the current session.
    async fn logout(&self) -> Result<(), crate::SynapticError>;

    /// Return `true` if the adapter currently holds valid credentials.
    fn is_authenticated(&self) -> bool;
}

/// Thread management.
#[async_trait]
pub trait Threading: ChannelAdapter {
    /// Create a new thread under `parent`, returning the new thread ID.
    async fn create_thread(
        &self,
        parent: &str,
        title: &str,
    ) -> Result<String, crate::SynapticError>;

    /// Post `content` as a reply inside an existing thread.
    async fn reply_in_thread(
        &self,
        thread_id: &str,
        content: &str,
    ) -> Result<(), crate::SynapticError>;
}

/// Group / channel management.
#[async_trait]
pub trait Groups: ChannelAdapter {
    /// List all groups visible to the adapter.
    async fn list_groups(&self) -> Result<Vec<GroupInfo>, crate::SynapticError>;

    /// Return the access policy applied to inbound group messages.
    fn group_policy(&self) -> GroupPolicy;
}

/// Emoji reactions.
#[async_trait]
pub trait Reactions: ChannelAdapter {
    /// Add an emoji reaction to message `msg_id`.
    async fn add_reaction(&self, msg_id: &str, emoji: &str) -> Result<(), crate::SynapticError>;

    /// Remove a previously added emoji reaction.
    async fn remove_reaction(&self, msg_id: &str, emoji: &str) -> Result<(), crate::SynapticError>;
}

/// @mention handling (synchronous — no I/O required).
pub trait Mentions: ChannelAdapter {
    /// Return `true` if the bot/agent is mentioned in `envelope`.
    fn is_mentioned(&self, envelope: &MessageEnvelope) -> bool;

    /// Extract all @mentions from a raw message string.
    fn extract_mentions(&self, content: &str) -> Vec<Mention>;
}

/// Platform-specific actions (pin, kick, ban, …).
#[async_trait]
pub trait PlatformActions: ChannelAdapter {
    /// Return the list of actions this adapter supports.
    fn available_actions(&self) -> Vec<ActionDef>;

    /// Execute a named action with the given parameters.
    async fn execute_action(
        &self,
        action: &str,
        params: Value,
    ) -> Result<Value, crate::SynapticError>;
}

/// Configuration validation.
#[async_trait]
pub trait ChannelConfig: ChannelAdapter {
    /// Return a JSON Schema describing the expected configuration.
    fn config_schema(&self) -> Value;

    /// Validate `config` against the schema, returning an error on failure.
    async fn validate_config(&self, config: &Value) -> Result<(), crate::SynapticError>;
}

/// Health checking.
#[async_trait]
pub trait ChannelHealth: ChannelAdapter {
    /// Perform a health check and return the current status.
    async fn health_check(&self) -> HealthStatus;
}
