use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::SynapticError;
use tokio::sync::RwLock;

use crate::{Event, EventAction, EventFilter, EventSubscriber};

/// An event subscriber that records all received events for later inspection, useful for testing.
#[derive(Default, Clone)]
pub struct RecordingCallback {
    events: Arc<RwLock<Vec<Event>>>,
}

impl RecordingCallback {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn events(&self) -> Vec<Event> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl EventSubscriber for RecordingCallback {
    fn subscriptions(&self) -> Vec<EventFilter> {
        vec![EventFilter::All]
    }

    async fn handle(&self, event: &mut Event) -> Result<EventAction, SynapticError> {
        self.events.write().await.push(event.clone());
        Ok(EventAction::Continue)
    }

    fn name(&self) -> &str {
        "RecordingCallback"
    }
}
