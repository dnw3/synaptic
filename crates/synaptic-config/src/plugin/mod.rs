//! Plugin system: Plugin trait, PluginManifest, PluginRegistry.
//!
//! Requires the `plugin` feature flag.

mod diagnostics;
mod manifest;
mod plugin_api;
mod plugin_trait;
mod registry;
mod service;

pub use diagnostics::*;
pub use manifest::*;
pub use plugin_api::*;
pub use plugin_trait::*;
pub use registry::*;
pub use service::*;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
                runtime: PluginRuntimeKind::Builtin,
                trust_tier: PluginTrustTier::CoreBuiltin,
                permissions: Vec::new(),
                declared_capabilities: Vec::new(),
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
            runtime: PluginRuntimeKind::Builtin,
            trust_tier: PluginTrustTier::CoreBuiltin,
            permissions: Vec::new(),
            declared_capabilities: Vec::new(),
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

    struct EchoGatewayMethod;

    #[async_trait::async_trait]
    impl PluginGatewayMethodHandler for EchoGatewayMethod {
        async fn invoke(
            &self,
            method: &str,
            params: serde_json::Value,
        ) -> Result<serde_json::Value, synaptic_core::SynapticError> {
            Ok(json!({
                "method": method,
                "params": params,
            }))
        }
    }

    struct NullProviderFactory;

    #[async_trait::async_trait]
    impl PluginProviderFactory for NullProviderFactory {}

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

    #[test]
    fn manifest_runtime_permissions_and_declared_capabilities_roundtrip() {
        let manifest = PluginManifest {
            name: "activity-inspector".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            author: None,
            license: None,
            capabilities: vec![PluginCapability::Tools, PluginCapability::Hooks],
            slot: None,
            runtime: PluginRuntimeKind::Builtin,
            trust_tier: PluginTrustTier::OfficialPlugin,
            permissions: vec![PluginPermission::GatewayMethod],
            declared_capabilities: vec![DeclaredCapability {
                kind: DeclaredCapabilityKind::GatewayMethod,
                name: "activity.recent".into(),
                scopes: vec!["operator.read".into()],
                experimental: false,
            }],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: PluginManifest = serde_json::from_str(&json).unwrap();

        assert!(matches!(parsed.runtime, PluginRuntimeKind::Builtin));
        assert!(matches!(parsed.trust_tier, PluginTrustTier::OfficialPlugin));
        assert_eq!(parsed.permissions, vec![PluginPermission::GatewayMethod]);
        assert_eq!(parsed.declared_capabilities.len(), 1);
        assert_eq!(parsed.declared_capabilities[0].name, "activity.recent");
    }

    #[test]
    fn registry_tracks_plugin_diagnostics() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);

        registry.record_diagnostic(
            "activity-inspector",
            PluginDiagnostic {
                level: PluginDiagnosticLevel::Warn,
                code: "missing-scope".into(),
                message: "scope missing".into(),
                subject: Some("activity.recent".into()),
            },
        );

        let diagnostics = registry
            .plugin_diagnostics("activity-inspector")
            .expect("diagnostics should exist");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "missing-scope");
    }

    #[tokio::test]
    async fn plugin_api_records_gateway_and_provider_capabilities() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);
        {
            let mut api = PluginApi::new(&mut registry, "activity-inspector");
            api.register_gateway_method("activity.recent", vec!["operator.read".into()]);
            api.register_provider("activity-model");
        }

        let registrations = registry
            .plugin_registrations("activity-inspector")
            .expect("registrations should exist");
        assert_eq!(registrations.gateway_methods, vec!["activity.recent"]);
        assert_eq!(registrations.providers, vec!["activity-model"]);

        let declared = registry
            .declared_capabilities("activity-inspector")
            .expect("declared capabilities should exist");
        assert_eq!(declared.len(), 2);
    }

    #[tokio::test]
    async fn plugin_api_registers_gateway_handlers_and_provider_factories() {
        let bus = Arc::new(EventBus::new());
        let mut registry = PluginRegistry::new(bus);
        {
            let mut api = PluginApi::new(&mut registry, "activity-inspector");
            api.register_gateway_method_handler(
                "activity.recent",
                vec!["operator.read".into()],
                Arc::new(EchoGatewayMethod),
            );
            api.register_provider_factory("activity-model", Arc::new(NullProviderFactory));
        }

        let gateway_methods = registry.plugin_gateway_methods("activity-inspector");
        assert_eq!(gateway_methods.len(), 1);
        assert_eq!(gateway_methods[0].name, "activity.recent");
        assert_eq!(gateway_methods[0].scopes, vec!["operator.read".to_string()]);
        let gateway_output = gateway_methods[0]
            .handler
            .invoke("activity.recent", json!({ "limit": 3 }))
            .await
            .unwrap();
        assert_eq!(gateway_output["method"], "activity.recent");
        assert_eq!(gateway_output["params"]["limit"], 3);

        let providers = registry.plugin_providers("activity-inspector");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "activity-model");
        assert!(providers[0]
            .factory
            .create_chat_model()
            .await
            .unwrap()
            .is_none());
    }
}
