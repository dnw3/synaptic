//! PluginApi — scoped registration interface for plugins.

use std::sync::Arc;
use synaptic_core::Tool;
use synaptic_events::EventSubscriber;
use synaptic_memory::MemoryProvider;
use synaptic_middleware::Interceptor;

use super::{PluginRegistry, Service};

/// Scoped registration API handed to each plugin during `register()`.
/// All registrations are automatically tagged with the `plugin_id`.
pub struct PluginApi<'a> {
    registry: &'a mut PluginRegistry,
    plugin_id: String,
}

impl<'a> PluginApi<'a> {
    pub fn new(registry: &'a mut PluginRegistry, plugin_id: impl Into<String>) -> Self {
        Self {
            registry,
            plugin_id: plugin_id.into(),
        }
    }

    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        self.registry.register_tool(tool);
    }

    /// Register event subscriber. Lower priority values execute first.
    pub fn register_event_subscriber(&self, subscriber: Arc<dyn EventSubscriber>, priority: i32) {
        self.registry.register_event_subscriber(
            subscriber,
            priority,
            format!("plugin:{}", self.plugin_id),
        );
    }

    pub fn register_memory(&mut self, provider: Arc<dyn MemoryProvider>) {
        self.registry.set_memory_slot(&self.plugin_id, provider);
    }

    pub fn register_service(&mut self, service: Box<dyn Service>) {
        self.registry.register_service(service);
    }

    pub fn register_interceptor(&mut self, interceptor: Arc<dyn Interceptor>) {
        self.registry.register_interceptor(interceptor);
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}
