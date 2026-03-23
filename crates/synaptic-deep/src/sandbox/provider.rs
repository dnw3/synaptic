use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;

use super::types::*;
use crate::backend::Backend;

/// Request to create a new sandbox instance.
#[derive(Debug, Clone)]
pub struct SandboxCreateRequest {
    pub scope_key: String,
    pub workspace: SandboxWorkspace,
    pub security: SandboxSecurityConfig,
    pub resources: SandboxResourceLimits,
    pub extra_mounts: Vec<BindMount>,
    pub setup_command: Option<String>,
    pub env: HashMap<String, String>,
}

/// A running sandbox instance with its Backend.
pub struct SandboxInstance {
    pub runtime_id: String,
    pub backend: Arc<dyn Backend>,
    pub info: SandboxInstanceInfo,
}

/// Metadata about a sandbox instance (serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstanceInfo {
    pub runtime_id: String,
    pub provider_id: String,
    pub runtime_label: String,
    pub scope_key: String,
    pub image: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

/// Status of a sandbox instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxStatus {
    Running,
    Stopped,
    NotFound,
}

/// Provider that manages sandbox lifecycle (create, destroy, status, list).
#[async_trait]
pub trait SandboxProvider: Send + Sync {
    /// Unique identifier for this provider (e.g. "docker", "ssh").
    fn id(&self) -> &str;

    /// Create a new sandbox instance from the given request.
    async fn create(&self, req: SandboxCreateRequest) -> Result<SandboxInstance, SynapticError>;

    /// Destroy a sandbox instance by its runtime ID.
    async fn destroy(&self, runtime_id: &str) -> Result<(), SynapticError>;

    /// Query the status of a sandbox instance.
    async fn status(&self, runtime_id: &str) -> Result<SandboxStatus, SynapticError>;

    /// List all sandbox instances managed by this provider.
    async fn list(&self) -> Result<Vec<SandboxInstanceInfo>, SynapticError>;
}

/// Registry of available sandbox providers, keyed by provider ID.
pub struct SandboxProviderRegistry {
    providers: HashMap<String, Arc<dyn SandboxProvider>>,
}

impl SandboxProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider. Overwrites any existing provider with the same ID.
    pub fn register(&mut self, provider: Arc<dyn SandboxProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    /// Look up a provider by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn SandboxProvider>> {
        self.providers.get(id).cloned()
    }

    /// List all registered provider IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for SandboxProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
