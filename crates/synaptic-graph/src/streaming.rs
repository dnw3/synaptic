//! Streaming output abstraction for agent execution.
//!
//! Provides a trait that platform adapters implement to receive real-time
//! updates as an agent generates a response (tokens, tool calls, completion).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Display metadata for a tool call, resolved by the product layer.
///
/// Carries human-friendly info (emoji, label, detail) so rendering surfaces
/// don't need to parse tool args themselves.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolDisplayMeta {
    /// Emoji indicator (e.g. "⚡" "📖" "✏️")
    pub emoji: String,
    /// Human-readable label (e.g. "Execute" "Read File")
    pub label: String,
    /// Present continuous verb for spinners (e.g. "executing" "reading")
    pub verb: String,
    /// Extracted summary from args (e.g. "ls -la ~/..." or "src/main.rs")
    pub detail: String,
}

/// Information about a tool call, passed to streaming output.
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    /// Tool name (e.g. "read_file", "task", "memory_search").
    pub name: String,
    /// Tool call ID (for dedup).
    pub id: String,
    /// Tool arguments as JSON string (for display purposes).
    pub args: String,
    /// Display metadata, resolved by the product layer. None if not resolved.
    pub display: Option<ToolDisplayMeta>,
}

/// Metadata about a completed agent execution.
#[derive(Debug, Clone, Default)]
pub struct CompletionMeta {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub duration_ms: u64,
    pub request_id: Option<String>,
}

/// Streaming output trait for real-time agent response rendering.
///
/// Implementors receive incremental updates as the agent generates a response,
/// enabling real-time message editing in chat platforms (e.g. Lark, Telegram).
#[async_trait]
pub trait StreamingOutput: Send + Sync {
    /// Called when new text content is generated (incremental delta).
    async fn on_token(&self, token: &str);

    /// Called when the agent invokes a tool.
    async fn on_tool_call(&self, info: &ToolCallInfo);

    /// Called when the agent finishes successfully with optional metadata.
    async fn on_complete(&self, full_response: &str, meta: Option<&CompletionMeta>);

    /// Called on error.
    async fn on_error(&self, error: &str);

    /// Called when reasoning/thinking content is generated.
    async fn on_reasoning(&self, _content: &str) {}

    /// Called when a tool execution completes with result.
    async fn on_tool_result(&self, _name: &str, _content: &str) {}

    /// Periodic heartbeat during long executions (e.g. tool runs >15s).
    async fn on_heartbeat(&self) {}
}
