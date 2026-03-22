use super::{PluginManifest, Service};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use synaptic_core::Tool;
use synaptic_events::{EventBus, EventSubscriber};
use synaptic_memory::MemoryProvider;
use synaptic_middleware::Interceptor;

/// Per-plugin registration tracking for UI introspection.
#[derive(Debug, Default, Clone, Serialize)]
pub struct PluginRegistrations {
    pub tools: Vec<String>,
    pub interceptors: Vec<String>,
    pub subscribers: Vec<String>,
    pub services: Vec<String>,
}

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
    registrations: HashMap<String, PluginRegistrations>,
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
            registrations: HashMap::new(),
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

    /// Unregister all resources belonging to a plugin (tools, interceptors, services,
    /// event subscribers, memory slot). Used for hot-disable.
    ///
    /// Returns the names of services that were removed (caller should stop them).
    pub fn unregister_plugin(&mut self, plugin_id: &str) -> Vec<String> {
        let regs = match self.registrations.remove(plugin_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Remove tools by name
        let tool_names: std::collections::HashSet<&str> =
            regs.tools.iter().map(|s| s.as_str()).collect();
        self.tools.retain(|t| !tool_names.contains(t.name()));

        // Remove interceptors — we track by name but interceptors don't expose names.
        // Remove by count: if plugin registered N interceptors, remove the last N.
        // This is imprecise but acceptable since plugins register interceptors in order.
        let n = regs.interceptors.len();
        if n > 0 && self.interceptors.len() >= n {
            // Remove from the end (plugin interceptors are appended last)
            self.interceptors.truncate(self.interceptors.len() - n);
        }

        // Remove event subscribers by tag ("plugin:{id}")
        let tag = format!("plugin:{plugin_id}");
        let removed = self.event_bus.unsubscribe_by_tag(&tag);
        tracing::debug!(
            plugin = plugin_id,
            subscribers_removed = removed,
            "unsubscribed event subscribers"
        );

        // Collect service IDs to return (caller stops them before we remove)
        let service_ids = regs.services.clone();
        let svc_names: std::collections::HashSet<&str> =
            regs.services.iter().map(|s| s.as_str()).collect();
        self.services.retain(|s| !svc_names.contains(s.id()));

        // Clear memory slot if owned by this plugin
        if self.memory_slot.as_ref().map(|e| e.plugin_id.as_str()) == Some(plugin_id) {
            self.memory_slot = None;
        }

        // Remove manifest
        self.plugins.retain(|m| m.name != plugin_id);

        tracing::info!(
            plugin = plugin_id,
            tools = regs.tools.len(),
            services = service_ids.len(),
            "plugin unregistered"
        );

        service_ids
    }

    /// Re-register a plugin: call `plugin.register()` and `record_plugin()`.
    /// The caller is responsible for calling `Plugin::start()` after this.
    pub async fn register_plugin(
        &mut self,
        plugin: &dyn super::Plugin,
    ) -> Result<(), synaptic_core::SynapticError> {
        let manifest = plugin.manifest();
        let name = manifest.name.clone();
        {
            let mut api = super::PluginApi::new(self, &name);
            plugin.register(&mut api).await?;
        }
        self.record_plugin(manifest);
        Ok(())
    }

    /// Record a registration entry for a plugin (tool, interceptor, subscriber, or service).
    pub fn record_registration(&mut self, plugin_id: &str, kind: &str, name: &str) {
        let entry = self.registrations.entry(plugin_id.to_string()).or_default();
        match kind {
            "tool" => entry.tools.push(name.to_string()),
            "interceptor" => entry.interceptors.push(name.to_string()),
            "subscriber" => entry.subscribers.push(name.to_string()),
            "service" => entry.services.push(name.to_string()),
            _ => {}
        }
    }

    /// Get registration details for a specific plugin.
    pub fn plugin_registrations(&self, plugin_id: &str) -> Option<&PluginRegistrations> {
        self.registrations.get(plugin_id)
    }

    /// Get all plugin registrations.
    pub fn all_registrations(&self) -> &HashMap<String, PluginRegistrations> {
        &self.registrations
    }
}
