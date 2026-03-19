//! Unified event system for agent lifecycle notifications.
//! Provides EventBus with 5 dispatch modes and EventSubscriber trait.

mod bus;
mod subscriber;
mod types;

pub use bus::*;
pub use subscriber::*;
pub use types::*;

pub mod observers;
pub mod prometheus;

/// Backward-compatible alias: the old `synaptic::callbacks` types now live
/// in `synaptic_events::observers`.
pub use observers as callbacks;
