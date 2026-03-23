//! Plugin system: Plugin trait, PluginManifest, PluginRegistry.
//!
//! Requires the `plugin` feature flag.

mod manifest;
mod plugin_api;
mod plugin_trait;
mod registry;
mod service;

pub use manifest::*;
pub use plugin_api::*;
pub use plugin_trait::*;
pub use registry::*;
pub use service::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use synaptic_events::EventBus;

    struct TestPlugin;

    #[async_trait::async_trait]
    impl Plugin for TestPlugin {
        fn manifest(&self) -> PluginManifest {
            PluginManifest {
                name: "test".into(),
                version: "0.1.0".into(),
                description: "test plugin".into(),
                author: None,
                license: None,
                capabilities: vec![PluginCapability::Tools],
                slot: None,
            }
        }
        async fn register(
            &self,
            _api: &mut PluginApi<'_>,
        ) -> Result<(), synaptic_core::SynapticError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn plugin_lifecycle() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);
        let plugin = TestPlugin;
        let manifest = plugin.manifest();
        let mut api = PluginApi::new(&mut registry, &manifest.name);
        plugin.register(&mut api).await.unwrap();
        registry.record_plugin(manifest);
        assert_eq!(registry.plugins().len(), 1);
        assert_eq!(registry.plugins()[0].name, "test");
    }

    #[test]
    fn manifest_with_slot_serializes() {
        let manifest = PluginManifest {
            name: "memory-plugin".into(),
            version: "1.0.0".into(),
            description: "A memory plugin".into(),
            author: None,
            license: None,
            capabilities: vec![PluginCapability::Memory],
            slot: Some(PluginSlot::Memory),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("\"slot\":\"memory\""));
        assert!(json.contains("\"memory\""));
    }

    #[test]
    fn manifest_without_slot_defaults_none() {
        let json = r#"{
            "name": "my-plugin",
            "version": "0.1.0",
            "description": "desc",
            "capabilities": ["tools"]
        }"#;
        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.slot.is_none());
        assert_eq!(manifest.name, "my-plugin");
    }

    // -----------------------------------------------------------------------
    // MemoryProvider mock
    // -----------------------------------------------------------------------

    struct NoopMemoryProvider;

    #[async_trait::async_trait]
    impl synaptic_memory::MemoryProvider for NoopMemoryProvider {
        async fn add_message(
            &self,
            _session_key: &str,
            _role: &str,
            _content: &str,
        ) -> Result<(), synaptic_core::SynapticError> {
            Ok(())
        }

        async fn record_usage(
            &self,
            _session_key: &str,
            _context_uris: &[String],
            _skill_uris: &[String],
        ) -> Result<(), synaptic_core::SynapticError> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<synaptic_memory::MemoryResult>, synaptic_core::SynapticError> {
            Ok(vec![])
        }

        async fn search(
            &self,
            _query: &str,
            _session_key: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<synaptic_memory::MemoryResult>, synaptic_core::SynapticError> {
            Ok(vec![])
        }

        async fn commit(
            &self,
            _session_key: &str,
        ) -> Result<synaptic_memory::CommitResult, synaptic_core::SynapticError> {
            Ok(synaptic_memory::CommitResult::default())
        }

        async fn add_resource(&self, _uri: &str) -> Result<(), synaptic_core::SynapticError> {
            Ok(())
        }

        async fn get_profile(
            &self,
            _user_id: &str,
        ) -> Result<Option<String>, synaptic_core::SynapticError> {
            Ok(None)
        }
    }

    // -----------------------------------------------------------------------
    // Interceptor mock
    // -----------------------------------------------------------------------

    struct NoopInterceptor;

    #[async_trait::async_trait]
    impl synaptic_middleware::Interceptor for NoopInterceptor {}

    // -----------------------------------------------------------------------
    // Service mock
    // -----------------------------------------------------------------------

    struct NoopService;

    #[async_trait::async_trait]
    impl Service for NoopService {
        fn id(&self) -> &str {
            "noop"
        }

        async fn start(&self) -> Result<(), synaptic_core::SynapticError> {
            Ok(())
        }

        async fn health_check(&self) -> bool {
            true
        }

        async fn stop(&self) {}
    }

    #[tokio::test]
    async fn registry_memory_slot_exclusive() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);

        // Initially empty
        assert!(registry.memory_slot().is_none());
        assert!(registry.memory_slot_owner().is_none());

        // Set for the first plugin
        let provider: Arc<dyn synaptic_memory::MemoryProvider> = Arc::new(NoopMemoryProvider);
        registry.set_memory_slot("plugin-a", provider);

        assert!(registry.memory_slot().is_some());
        assert_eq!(registry.memory_slot_owner(), Some("plugin-a"));

        // Replace with a second plugin (warning logged, but slot replaced)
        let provider2: Arc<dyn synaptic_memory::MemoryProvider> = Arc::new(NoopMemoryProvider);
        registry.set_memory_slot("plugin-b", provider2);

        assert_eq!(registry.memory_slot_owner(), Some("plugin-b"));
    }

    #[tokio::test]
    async fn registry_interceptors() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);

        assert_eq!(registry.interceptors().len(), 0);

        registry.register_interceptor(Arc::new(NoopInterceptor));
        registry.register_interceptor(Arc::new(NoopInterceptor));

        assert_eq!(registry.interceptors().len(), 2);

        let taken = registry.take_interceptors();
        assert_eq!(taken.len(), 2);
        assert_eq!(registry.interceptors().len(), 0);
    }

    #[tokio::test]
    async fn plugin_api_scoped_registration() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);
        {
            let mut api = PluginApi::new(&mut registry, "test-plugin");
            struct FakeTool;
            #[async_trait::async_trait]
            impl synaptic_core::Tool for FakeTool {
                fn name(&self) -> &'static str {
                    "fake"
                }
                fn description(&self) -> &'static str {
                    "fake"
                }
                async fn call(
                    &self,
                    _: serde_json::Value,
                ) -> Result<serde_json::Value, synaptic_core::SynapticError> {
                    Ok(serde_json::Value::Null)
                }
            }
            api.register_tool(Arc::new(FakeTool));
        }
        assert_eq!(registry.tools().len(), 1);
    }

    #[tokio::test]
    async fn plugin_registrations_tracked() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);
        {
            let mut api = PluginApi::new(&mut registry, "test-plugin");
            struct FakeTool;
            #[async_trait::async_trait]
            impl synaptic_core::Tool for FakeTool {
                fn name(&self) -> &'static str {
                    "fake"
                }
                fn description(&self) -> &'static str {
                    "fake"
                }
                async fn call(
                    &self,
                    _: serde_json::Value,
                ) -> Result<serde_json::Value, synaptic_core::SynapticError> {
                    Ok(serde_json::Value::Null)
                }
            }
            api.register_tool(Arc::new(FakeTool));
            api.register_service(Box::new(NoopService));
            api.register_interceptor(Arc::new(NoopInterceptor));
        }
        let regs = registry.plugin_registrations("test-plugin").unwrap();
        assert_eq!(regs.tools, vec!["fake"]);
        assert_eq!(regs.services, vec!["noop"]);
        assert_eq!(regs.interceptors, vec!["NoopInterceptor"]);
        assert!(regs.subscribers.is_empty());
        assert!(registry.all_registrations().len() == 1);
    }

    #[tokio::test]
    async fn registry_services() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);

        assert_eq!(registry.services().len(), 0);

        registry.register_service(Box::new(NoopService));
        registry.register_service(Box::new(NoopService));

        assert_eq!(registry.services().len(), 2);

        let taken = registry.take_services();
        assert_eq!(taken.len(), 2);
        assert_eq!(registry.services().len(), 0);
    }
}
