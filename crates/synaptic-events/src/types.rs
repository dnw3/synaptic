//! Core event types for the Synaptic event system.

use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use synaptic_core::SynapticError;

// ---------------------------------------------------------------------------
// DispatchMode
// ---------------------------------------------------------------------------

/// Determines how subscribers receive and can influence an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DispatchMode {
    /// Fire-and-forget; subscribers cannot modify or cancel the event.
    Parallel,
    /// Subscribers run in order; each may modify payload or cancel the event.
    Sequential,
    /// Subscribers can short-circuit the operation with a result value.
    Intercept,
    /// Triggered on failures; subscribers may retry or intercept.
    ErrorPath,
    /// Blocking hot-path; subscribers run synchronously before proceeding.
    Synchronous,
}

// ---------------------------------------------------------------------------
// EventKind
// ---------------------------------------------------------------------------

/// All 28 event kinds in the Synaptic event system.
///
/// Variants are fieldless — payloads are carried by `Event::payload` (a `Value`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    // Immutable — Parallel
    GatewayStart,
    GatewayStop,
    SessionStart,
    SessionEnd,
    MessageReceived,
    MessageSent,
    LlmInput,
    LlmOutput,
    AfterToolCall,
    AgentEnd,
    SubagentEnded,
    AfterCompaction,
    AfterFileOp,
    AfterCommand,

    // Mutable — Sequential
    BeforeModelResolve,
    BeforePromptBuild,
    MessageSending,
    SubagentSpawning,
    BeforeCompaction,

    // Intercept
    BeforeToolCall,
    BeforeModelCall,
    BeforeFileOp,
    BeforeCommand,

    // ErrorPath
    OnToolError,
    OnModelError,

    // Synchronous
    BeforeMessageWrite,
    ToolResultPersist,

    // Infrastructure — Parallel
    ConfigReloaded,
}

impl EventKind {
    /// Returns the dispatch mode for this event kind.
    pub fn dispatch_mode(&self) -> DispatchMode {
        match self {
            // Parallel (immutable)
            EventKind::GatewayStart
            | EventKind::GatewayStop
            | EventKind::SessionStart
            | EventKind::SessionEnd
            | EventKind::MessageReceived
            | EventKind::MessageSent
            | EventKind::LlmInput
            | EventKind::LlmOutput
            | EventKind::AfterToolCall
            | EventKind::AgentEnd
            | EventKind::SubagentEnded
            | EventKind::AfterCompaction
            | EventKind::AfterFileOp
            | EventKind::AfterCommand
            | EventKind::ConfigReloaded => DispatchMode::Parallel,

            // Sequential (mutable)
            EventKind::BeforeModelResolve
            | EventKind::BeforePromptBuild
            | EventKind::MessageSending
            | EventKind::SubagentSpawning
            | EventKind::BeforeCompaction => DispatchMode::Sequential,

            // Intercept
            EventKind::BeforeToolCall
            | EventKind::BeforeModelCall
            | EventKind::BeforeFileOp
            | EventKind::BeforeCommand => DispatchMode::Intercept,

            // ErrorPath
            EventKind::OnToolError | EventKind::OnModelError => DispatchMode::ErrorPath,

            // Synchronous
            EventKind::BeforeMessageWrite | EventKind::ToolResultPersist => {
                DispatchMode::Synchronous
            }
        }
    }

    /// Returns all 28 event kind variants.
    pub fn all() -> Vec<EventKind> {
        vec![
            EventKind::GatewayStart,
            EventKind::GatewayStop,
            EventKind::SessionStart,
            EventKind::SessionEnd,
            EventKind::MessageReceived,
            EventKind::MessageSent,
            EventKind::LlmInput,
            EventKind::LlmOutput,
            EventKind::AfterToolCall,
            EventKind::AgentEnd,
            EventKind::SubagentEnded,
            EventKind::AfterCompaction,
            EventKind::AfterFileOp,
            EventKind::AfterCommand,
            EventKind::BeforeModelResolve,
            EventKind::BeforePromptBuild,
            EventKind::MessageSending,
            EventKind::SubagentSpawning,
            EventKind::BeforeCompaction,
            EventKind::BeforeToolCall,
            EventKind::BeforeModelCall,
            EventKind::BeforeFileOp,
            EventKind::BeforeCommand,
            EventKind::OnToolError,
            EventKind::OnModelError,
            EventKind::BeforeMessageWrite,
            EventKind::ToolResultPersist,
            EventKind::ConfigReloaded,
        ]
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Serialise via serde to get the snake_case string.
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{self:?}"));
        f.write_str(&s)
    }
}

impl FromStr for EventKind {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Wrap in quotes so serde_json treats it as a JSON string.
        serde_json::from_str(&format!("\"{}\"", s))
    }
}

// ---------------------------------------------------------------------------
// EventMeta
// ---------------------------------------------------------------------------

/// Metadata attached to every event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventMeta {
    /// Optional correlation ID linking the event to a specific request.
    pub request_id: Option<String>,
    /// Unix timestamp in milliseconds when the event was created.
    pub timestamp: u64,
    /// Human-readable name of the component that emitted the event.
    pub source: String,
}

impl EventMeta {
    /// Creates a new `EventMeta` with an auto-generated timestamp.
    pub fn new(source: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            request_id: None,
            timestamp,
            source: source.into(),
        }
    }

    /// Builder — attach a request ID.
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// A single event flowing through the Synaptic event bus.
#[derive(Clone, Debug)]
pub struct Event {
    /// Discriminant that identifies what happened.
    pub kind: EventKind,
    /// Arbitrary JSON payload; schema is convention-based per kind.
    pub payload: Value,
    /// Provenance and correlation metadata.
    pub metadata: EventMeta,
}

impl Event {
    /// Creates a new event with the default source (`"unknown"`).
    pub fn new(kind: EventKind, payload: Value) -> Self {
        Self {
            kind,
            payload,
            metadata: EventMeta::new("unknown"),
        }
    }

    /// Builder — set the source component name.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.source = source.into();
        self
    }

    /// Builder — attach a request ID.
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_request_id(id);
        self
    }
}

// ---------------------------------------------------------------------------
// EventAction
// ---------------------------------------------------------------------------

/// The action a subscriber returns after processing an event.
#[derive(Debug)]
pub enum EventAction {
    /// No-op — processing continues unchanged.
    Continue,
    /// The subscriber mutated the payload in place; use the updated event.
    Modify,
    /// Cancel the operation that triggered the event.
    Cancel,
    /// Short-circuit with the provided value (for `Intercept` events).
    Intercept(Value),
    /// Re-execute the failed operation (valid on `ErrorPath` events).
    Retry,
    /// Propagate the given error up the call stack.
    Error(SynapticError),
}

// ---------------------------------------------------------------------------
// EmitResult
// ---------------------------------------------------------------------------

/// The outcome returned by the event bus after all subscribers have run.
#[derive(Debug)]
pub enum EmitResult {
    /// All subscribers ran; continue with the (possibly modified) event.
    Proceed,
    /// A subscriber cancelled the operation.
    Cancelled,
    /// A subscriber short-circuited with this value.
    Intercepted(Value),
    /// A subscriber requested a retry of the failed operation.
    Retry,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn event_kind_is_hashable() {
        let mut set = HashSet::new();
        set.insert(EventKind::GatewayStart);
        set.insert(EventKind::GatewayStart);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn event_kind_dispatch_mode() {
        assert_eq!(
            EventKind::GatewayStart.dispatch_mode(),
            DispatchMode::Parallel
        );
        assert_eq!(
            EventKind::BeforeToolCall.dispatch_mode(),
            DispatchMode::Intercept
        );
        assert_eq!(
            EventKind::OnToolError.dispatch_mode(),
            DispatchMode::ErrorPath
        );
        assert_eq!(
            EventKind::BeforeMessageWrite.dispatch_mode(),
            DispatchMode::Synchronous
        );
        assert_eq!(
            EventKind::BeforePromptBuild.dispatch_mode(),
            DispatchMode::Sequential
        );
        assert_eq!(
            EventKind::ConfigReloaded.dispatch_mode(),
            DispatchMode::Parallel
        );
    }

    #[test]
    fn event_kind_from_str_snake_case() {
        assert_eq!(
            "gateway_start".parse::<EventKind>().unwrap(),
            EventKind::GatewayStart
        );
        assert_eq!(
            "before_tool_call".parse::<EventKind>().unwrap(),
            EventKind::BeforeToolCall
        );
        assert_eq!(
            "on_tool_error".parse::<EventKind>().unwrap(),
            EventKind::OnToolError
        );
    }

    #[test]
    fn all_28_events_exist() {
        assert_eq!(EventKind::all().len(), 28);
    }

    #[test]
    fn event_meta_auto_timestamp() {
        let meta = EventMeta::new("test");
        assert!(meta.timestamp > 0);
        assert_eq!(meta.source, "test");
        assert!(meta.request_id.is_none());
    }

    #[test]
    fn event_builder() {
        let event = Event::new(EventKind::GatewayStart, serde_json::json!({"port": 3000}))
            .with_source("gateway")
            .with_request_id("req-123");
        assert_eq!(event.kind, EventKind::GatewayStart);
        assert_eq!(event.metadata.source, "gateway");
        assert_eq!(event.metadata.request_id.as_deref(), Some("req-123"));
    }
}
