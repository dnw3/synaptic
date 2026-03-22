//! Plugin system: Plugin trait, PluginManifest, PluginRegistry.
//!
//! Requires the `plugin` feature flag.

mod manifest;
mod plugin_trait;
mod registry;
mod service;

pub use manifest::*;
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
            _registry: &mut PluginRegistry,
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
        plugin.register(&mut registry).await.unwrap();
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
}
