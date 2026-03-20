use super::{PluginManifest, PluginRegistry};
use async_trait::async_trait;

pub struct PluginContext {
    pub data_dir: std::path::PathBuf,
}

#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> PluginManifest;
    async fn register(
        &self,
        registry: &mut PluginRegistry,
    ) -> Result<(), synaptic_core::SynapticError>;
    async fn start(&self, _ctx: PluginContext) -> Result<(), synaptic_core::SynapticError> {
        Ok(())
    }
    async fn stop(&self) -> Result<(), synaptic_core::SynapticError> {
        Ok(())
    }
}
