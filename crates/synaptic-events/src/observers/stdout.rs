use async_trait::async_trait;
use synaptic_core::SynapticError;

use crate::{Event, EventAction, EventFilter, EventSubscriber};

/// An event subscriber that prints events to stdout.
///
/// When `verbose` is true, additional detail is printed for each event.
pub struct StdOutCallbackHandler {
    verbose: bool,
}

impl StdOutCallbackHandler {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn verbose() -> Self {
        Self { verbose: true }
    }
}

impl Default for StdOutCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventSubscriber for StdOutCallbackHandler {
    fn subscriptions(&self) -> Vec<EventFilter> {
        vec![EventFilter::All]
    }

    async fn handle(&self, event: &mut Event) -> Result<EventAction, SynapticError> {
        let kind = format!("{:?}", event.kind);
        let request_id = event.metadata.request_id.as_deref().unwrap_or("?");

        if self.verbose {
            println!("[{kind}] request_id={request_id} payload={}", event.payload);
        } else {
            println!("[{kind}] request_id={request_id}");
        }
        Ok(EventAction::Continue)
    }

    fn name(&self) -> &str {
        "StdOutCallbackHandler"
    }
}
