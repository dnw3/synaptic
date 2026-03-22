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
        self.registry
            .record_registration(&self.plugin_id, "tool", tool.name());
        self.registry.register_tool(tool);
    }

    /// Register event subscriber. Lower priority values execute first.
    pub fn register_event_subscriber(
        &mut self,
        subscriber: Arc<dyn EventSubscriber>,
        priority: i32,
    ) {
        let name = subscriber.name().to_string();
        self.registry.register_event_subscriber(
            subscriber,
            priority,
            format!("plugin:{}", self.plugin_id),
        );
        self.registry
            .record_registration(&self.plugin_id, "subscriber", &name);
    }

    pub fn register_memory(&mut self, provider: Arc<dyn MemoryProvider>) {
        self.registry.set_memory_slot(&self.plugin_id, provider);
    }

    pub fn register_service(&mut self, service: Box<dyn Service>) {
        let id = service.id().to_string();
        self.registry.register_service(service);
        self.registry
            .record_registration(&self.plugin_id, "service", &id);
    }

    pub fn register_interceptor(&mut self, interceptor: Arc<dyn Interceptor>) {
        // Extract short name: "synapse::plugins::memory_recall::MemoryRecallInterceptor" → "MemoryRecallInterceptor"
        let short_name = interceptor
            .name()
            .rsplit("::")
            .next()
            .unwrap_or_else(|| interceptor.name())
            .to_owned();
        self.registry.register_interceptor(interceptor);
        self.registry
            .record_registration(&self.plugin_id, "interceptor", &short_name);
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}
