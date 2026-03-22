use super::{PluginManifest, Service};
use std::sync::Arc;
use synaptic_core::Tool;
use synaptic_events::{EventBus, EventSubscriber};
use synaptic_memory::MemoryProvider;
use synaptic_middleware::Interceptor;

struct MemorySlotEntry {
    plugin_id: String,
    provider: Arc<dyn MemoryProvider>,
}

pub struct PluginRegistry {
    plugins: Vec<PluginManifest>,
    tools: Vec<Arc<dyn Tool>>,
    event_bus: Arc<EventBus>,
    memory_slot: Option<MemorySlotEntry>,
    services: Vec<Box<dyn Service>>,
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl PluginRegistry {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            plugins: Vec::new(),
            tools: Vec::new(),
            event_bus,
            memory_slot: None,
            services: Vec::new(),
            interceptors: Vec::new(),
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

    /// Set the exclusive memory slot. Logs a warning if already occupied and replaces the entry.
    pub fn set_memory_slot(&mut self, plugin_id: &str, provider: Arc<dyn MemoryProvider>) {
        if let Some(ref existing) = self.memory_slot {
            tracing::warn!(
                existing_owner = %existing.plugin_id,
                new_owner = %plugin_id,
                "memory slot already occupied; replacing"
            );
        }
        self.memory_slot = Some(MemorySlotEntry {
            plugin_id: plugin_id.to_string(),
            provider,
        });
    }

    /// Returns a reference to the memory provider if one is registered.
    pub fn memory_slot(&self) -> Option<&Arc<dyn MemoryProvider>> {
        self.memory_slot.as_ref().map(|e| &e.provider)
    }

    /// Returns the plugin ID that owns the memory slot, if any.
    pub fn memory_slot_owner(&self) -> Option<&str> {
        self.memory_slot.as_ref().map(|e| e.plugin_id.as_str())
    }

    /// Register a managed lifecycle service.
    pub fn register_service(&mut self, service: Box<dyn Service>) {
        tracing::info!(service_id = %service.id(), "registered service via plugin");
        self.services.push(service);
    }

    pub fn services(&self) -> &[Box<dyn Service>] {
        &self.services
    }

    /// Take all registered services out of the registry, leaving it empty.
    pub fn take_services(&mut self) -> Vec<Box<dyn Service>> {
        std::mem::take(&mut self.services)
    }

    /// Register an interceptor for model/tool call interception.
    pub fn register_interceptor(&mut self, interceptor: Arc<dyn Interceptor>) {
        tracing::info!("registered interceptor via plugin");
        self.interceptors.push(interceptor);
    }

    pub fn interceptors(&self) -> &[Arc<dyn Interceptor>] {
        &self.interceptors
    }

    /// Take all registered interceptors out of the registry, leaving it empty.
    pub fn take_interceptors(&mut self) -> Vec<Arc<dyn Interceptor>> {
        std::mem::take(&mut self.interceptors)
    }

    /// Access the shared event bus.
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }
}
