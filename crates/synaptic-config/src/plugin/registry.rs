use super::PluginManifest;
use std::sync::Arc;
use synaptic_core::Tool;
use synaptic_events::{EventBus, EventSubscriber};

pub struct PluginRegistry {
    plugins: Vec<PluginManifest>,
    tools: Vec<Arc<dyn Tool>>,
    event_bus: Arc<EventBus>,
}

impl PluginRegistry {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            plugins: Vec::new(),
            tools: Vec::new(),
            event_bus,
        }
    }

    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        tracing::info!(tool = tool.name(), "registered tool via plugin");
        self.tools.push(tool);
    }

    pub fn register_event_subscriber(
        &self,
        subscriber: Arc<dyn EventSubscriber>,
        priority: i32,
        source: impl Into<String>,
    ) {
        let source = source.into();
        tracing::info!(source = %source, "registered event subscriber via plugin");
        self.event_bus.subscribe(subscriber, priority, source);
    }

    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }
    pub fn plugins(&self) -> &[PluginManifest] {
        &self.plugins
    }

    pub fn record_plugin(&mut self, manifest: PluginManifest) {
        self.plugins.push(manifest);
    }
}
