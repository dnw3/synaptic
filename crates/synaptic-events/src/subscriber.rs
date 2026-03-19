//! EventSubscriber trait — the sole extension point for Synaptic events.

use async_trait::async_trait;

use crate::{Event, EventAction, EventKind};

// ---------------------------------------------------------------------------
// EventFilter
// ---------------------------------------------------------------------------

/// Filter for which events a subscriber wants to receive.
#[derive(Clone, Debug)]
pub enum EventFilter {
    /// Match exactly one event kind.
    Exact(EventKind),
    /// Match any of the given event kinds.
    AnyOf(Vec<EventKind>),
    /// Match every event kind.
    All,
}

impl EventFilter {
    /// Returns `true` if this filter matches the given `kind`.
    pub fn matches(&self, kind: &EventKind) -> bool {
        match self {
            Self::Exact(k) => k == kind,
            Self::AnyOf(kinds) => kinds.contains(kind),
            Self::All => true,
        }
    }
}

// ---------------------------------------------------------------------------
// EventSubscriber
// ---------------------------------------------------------------------------

/// The sole extension point for event-driven lifecycle hooks.
///
/// Implementors register themselves with an [`EventBus`] and receive events
/// matching their declared [`EventFilter`]s.
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Declare which event kinds this subscriber wants to receive.
    fn subscriptions(&self) -> Vec<EventFilter>;

    /// Called for each matching event.
    ///
    /// The `event` may be mutated in place; return [`EventAction::Modify`] to
    /// signal that the payload was changed.
    async fn handle(&self, event: &mut Event) -> Result<EventAction, synaptic_core::SynapticError>;

    /// Human-readable name used in diagnostics. Defaults to the type name.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}
