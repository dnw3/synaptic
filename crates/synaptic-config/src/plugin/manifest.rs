use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeKind {
    #[default]
    Builtin,
    Sidecar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginTrustTier {
    CoreBuiltin,
    OfficialPlugin,
    #[default]
    ThirdPartyPlugin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    GatewayMethod,
    Provider,
    HostFilesystem,
    HostEnvironment,
    NetworkEgress,
    LocalExec,
    SensitiveHook,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredCapabilityKind {
    Tool,
    Hook,
    GatewayMethod,
    Provider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredCapability {
    pub kind: DeclaredCapabilityKind,
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub experimental: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginSlot {
    Memory,
    ContextEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<PluginSlot>,
    #[serde(default)]
    pub runtime: PluginRuntimeKind,
    #[serde(default)]
    pub trust_tier: PluginTrustTier,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    #[serde(default)]
    pub declared_capabilities: Vec<DeclaredCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Tools,
    Hooks,
    Channels,
    Providers,
    HttpRoutes,
    Commands,
    Services,
    CanvasRenderers,
    Memory,
}
