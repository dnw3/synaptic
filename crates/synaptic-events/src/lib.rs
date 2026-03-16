//! Unified event system replacing AgentMiddleware + Hook patterns.
//! Provides EventBus with 5 dispatch modes and EventSubscriber trait.

mod bus;
mod subscriber;
mod types;

pub use bus::*;
pub use subscriber::*;
pub use types::*;
