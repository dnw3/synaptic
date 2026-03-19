mod registry;
mod task_tool;
mod types;

use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use synaptic_core::{ChatModel, Tool};
use tokio::sync::Semaphore;

use crate::backend::Backend;
use crate::ModelResolver;

// Re-export all public types
pub use registry::{BackgroundTaskRegistry, BackgroundTaskResult};
pub use task_tool::{LlmTaskTool, TaskOutputTool, TaskToolSpawner};
pub use types::{
    builtin_agent_def, expand_tool_group, filter_tools_by_allow_deny, resolve_agent_memory_dir,
    SubAgentDef,
};

use task_tool::TaskTool;

/// Middleware that provides a `task` tool for spawning child agents.
///
/// The `task` tool creates a child deep agent and invokes it with the given description.
/// Recursion is bounded by `max_depth`, concurrency by a semaphore.
pub struct SubAgentMiddleware {
    backend: Arc<dyn Backend>,
    model: Arc<dyn ChatModel>,
    max_depth: usize,
    current_depth: Arc<AtomicUsize>,
    custom_agents: Vec<SubAgentDef>,
    concurrency: Arc<Semaphore>,
    model_resolver: Option<Arc<dyn ModelResolver>>,
    bg_registry: Arc<BackgroundTaskRegistry>,
    tool_profiles: HashMap<String, Vec<String>>,
    /// Max concurrent children per agent type (0 = unlimited).
    max_children_per_agent: usize,
}

impl SubAgentMiddleware {
    pub fn new(
        backend: Arc<dyn Backend>,
        model: Arc<dyn ChatModel>,
        max_depth: usize,
        custom_agents: Vec<SubAgentDef>,
    ) -> Self {
        Self::with_concurrency(backend, model, max_depth, custom_agents, 3)
    }

    pub fn with_concurrency(
        backend: Arc<dyn Backend>,
        model: Arc<dyn ChatModel>,
        max_depth: usize,
        custom_agents: Vec<SubAgentDef>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            backend,
            model,
            max_depth,
            current_depth: Arc::new(AtomicUsize::new(0)),
            custom_agents,
            concurrency: Arc::new(Semaphore::new(max_concurrent)),
            model_resolver: None,
            bg_registry: Arc::new(BackgroundTaskRegistry::new()),
            tool_profiles: HashMap::new(),
            max_children_per_agent: 0,
        }
    }

    /// Set the maximum concurrent children per agent type (0 = unlimited).
    pub fn with_max_children_per_agent(mut self, max: usize) -> Self {
        self.max_children_per_agent = max;
        self
    }

    /// Set the model resolver for name-based model selection.
    pub fn with_model_resolver(mut self, resolver: Arc<dyn ModelResolver>) -> Self {
        self.model_resolver = Some(resolver);
        self
    }

    /// Set named tool profiles for sub-agent tool filtering.
    pub fn with_tool_profiles(mut self, profiles: HashMap<String, Vec<String>>) -> Self {
        self.tool_profiles = profiles;
        self
    }

    /// Get a reference to the background task registry.
    pub fn background_registry(&self) -> Arc<BackgroundTaskRegistry> {
        self.bg_registry.clone()
    }

    /// Create the `task` tool that spawns sub-agents.
    pub fn create_task_tool(&self) -> Arc<dyn Tool> {
        Arc::new(TaskTool {
            backend: self.backend.clone(),
            model: self.model.clone(),
            max_depth: self.max_depth,
            current_depth: self.current_depth.clone(),
            custom_agents: self.custom_agents.clone(),
            concurrency: self.concurrency.clone(),
            model_resolver: self.model_resolver.clone(),
            bg_registry: self.bg_registry.clone(),
            tool_profiles: self.tool_profiles.clone(),
            max_children_per_agent: self.max_children_per_agent,
        })
    }
}
