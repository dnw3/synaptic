mod manifest;
mod plugin;
mod registry;

pub use manifest::*;
pub use plugin::*;
pub use registry::*;

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
}
