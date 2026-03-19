//! Deep agent harness for Synaptic.
//!
//! Provides an opinionated agent harness that bundles filesystem tools,
//! subagent spawning, skills, memory, and auto-summarization — all
//! implemented as [`Interceptor`](synaptic_middleware::Interceptor).
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use synaptic_deep::{create_deep_agent, DeepAgentOptions, backend::StateBackend};
//!
//! let backend = Arc::new(StateBackend::new());
//! let options = DeepAgentOptions::new(backend);
//! let agent = create_deep_agent(model, options)?;
//! let result = agent.invoke(MessageState::with_messages(vec![
//!     Message::human("Write hello.txt"),
//! ])).await?;
//! ```

pub mod acp;
pub mod backend;
#[cfg(feature = "config-builder")]
pub mod builder;
pub mod middleware;
pub mod skill;
pub mod tools;

use std::collections::HashMap;
use std::sync::Arc;

use synaptic_core::{ChatModel, Store, SynapticError, Tool};
use synaptic_events::EventBus;
use synaptic_graph::{create_agent, AgentOptions, Checkpointer, CompiledGraph, MessageState};
use synaptic_macros::traceable;
use synaptic_middleware::Interceptor;

use backend::Backend;
pub use middleware::environment::{ChannelInfo, EnvironmentInfo, EnvironmentMiddleware};
pub use middleware::observability::AgentTracingMiddleware;
pub use middleware::reflection::{ReflectionConfig, ReflectionMiddleware};
pub use middleware::skills::{
    substitute_context_vars, CommandExecutor, InstallSpec, SkillDef, SkillHookEvent,
    SkillHooksExecutor, SkillOverride, SkillStatusReport, SkillTool, SubAgentSpawner,
};
pub use middleware::subagent::{
    builtin_agent_def, expand_tool_group, BackgroundTaskRegistry, BackgroundTaskResult,
    LlmTaskTool, SubAgentDef, TaskOutputTool, TaskToolSpawner,
};

#[cfg(feature = "config-builder")]
pub use builder::build_agent_from_config;

// ---------------------------------------------------------------------------
// ModelResolver — maps model aliases to ChatModel instances
// ---------------------------------------------------------------------------

/// Resolves model name aliases (e.g. "sonnet", "opus", "haiku") to
/// concrete [`ChatModel`] instances.
///
/// The framework defines the trait; the product layer implements it with
/// knowledge of available providers.
#[async_trait::async_trait]
pub trait ModelResolver: Send + Sync {
    async fn resolve(&self, name: &str) -> Result<Arc<dyn ChatModel>, SynapticError>;
}

/// Configuration for [`create_deep_agent`].
pub struct DeepAgentOptions {
    /// Backend for filesystem operations.
    pub backend: Arc<dyn Backend>,
    /// Optional system prompt prepended to all model calls.
    pub system_prompt: Option<String>,
    /// Additional tools beyond the built-in filesystem tools.
    pub tools: Vec<Arc<dyn Tool>>,
    /// Interceptors (model + tool call wrappers).
    pub interceptors: Vec<Arc<dyn Interceptor>>,
    /// Optional checkpointer for graph state persistence.
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    /// Optional store for runtime tool injection.
    pub store: Option<Arc<dyn Store>>,
    /// Maximum input tokens before summarization (default 128,000).
    pub max_input_tokens: usize,
    /// Fraction of max_input_tokens that triggers summarization (default 0.85).
    pub summarization_threshold: f64,
    /// Token count above which tool results are evicted to files (default 20,000).
    pub eviction_threshold: usize,
    /// Maximum nested subagent depth (default 3).
    pub max_subagent_depth: usize,
    /// Skills directories in the backend, ordered by priority (high first).
    /// Default: `[".claude/skills"]`.
    pub skills_dirs: Vec<String>,
    /// Memory file path in the backend (default "AGENTS.md").
    pub memory_file: Option<String>,
    /// Custom subagent definitions for the task tool.
    pub subagents: Vec<SubAgentDef>,
    /// Enable subagent spawning via task tool (default true).
    pub enable_subagents: bool,
    /// Enable filesystem tools (default true).
    pub enable_filesystem: bool,
    /// Enable skills middleware (default true).
    pub enable_skills: bool,
    /// Enable memory middleware (default true).
    pub enable_memory: bool,
    /// Enable parallel tool execution in ToolNode (default false).
    pub parallel_tools: bool,
    /// Command executor for resolving !`command` placeholders in skills.
    pub command_executor: Option<Arc<dyn CommandExecutor>>,
    /// Hooks executor for skill lifecycle events.
    pub hooks_executor: Option<Arc<dyn SkillHooksExecutor>>,
    /// Maximum concurrent sub-agents (default 3).
    pub max_concurrent_subagents: usize,
    /// Maximum concurrent children per agent type (0 = unlimited, default 0).
    pub max_children_per_agent: usize,
    /// Maximum agent iterations / turns (None = default 100).
    pub max_iterations: Option<usize>,
    /// Resolver for model name aliases (e.g. "sonnet" → model instance).
    pub model_resolver: Option<Arc<dyn ModelResolver>>,
    /// Max chars for skill descriptions in the system prompt (default 16000).
    pub skill_description_budget: usize,
    /// Per-skill overrides (enabled/env).
    pub skill_overrides: HashMap<String, SkillOverride>,
    /// Named tool profiles for sub-agents (e.g. "minimal" → ["read_file", "write_file"]).
    pub tool_profiles: HashMap<String, Vec<String>>,
    /// Session ID for context variable substitution in skills (e.g. `${CLAUDE_SESSION_ID}`).
    pub session_id: Option<String>,
    /// Runtime environment info for self-awareness injection. None = disabled.
    pub environment: Option<middleware::environment::EnvironmentInfo>,
    /// Optional product-specific "self" section text.
    pub self_section: Option<String>,
    /// Optional lightweight model for post-session reflection. None = disabled.
    pub reflection_model: Option<Arc<dyn ChatModel>>,
    /// Reflection configuration. Only used when `reflection_model` is Some.
    pub reflection_config: Option<ReflectionConfig>,
    /// Optional event bus for emitting lifecycle events (model calls, tool
    /// calls, agent end, etc.). Events are emitted natively from graph nodes.
    pub event_bus: Option<Arc<EventBus>>,
    /// Optional model name for event payloads (overrides model.profile()).
    pub model_name: Option<String>,
    /// Optional provider name for event payloads.
    pub provider_name: Option<String>,
    /// Optional channel name for context injection into events (e.g. "lark", "web").
    pub channel: Option<String>,
    /// Optional agent ID for context injection into events.
    pub agent_id: Option<String>,
}

impl DeepAgentOptions {
    /// Create options with the given backend and sensible defaults.
    pub fn new(backend: Arc<dyn Backend>) -> Self {
        Self {
            backend,
            system_prompt: None,
            tools: Vec::new(),
            interceptors: Vec::new(),
            checkpointer: None,
            store: None,
            max_input_tokens: 128_000,
            summarization_threshold: 0.85,
            eviction_threshold: 20_000,
            max_subagent_depth: 3,
            skills_dirs: vec![".claude/skills".to_string()],
            memory_file: Some("AGENTS.md".to_string()),
            subagents: Vec::new(),
            enable_subagents: true,
            enable_filesystem: true,
            enable_skills: true,
            enable_memory: true,
            parallel_tools: false,
            command_executor: None,
            hooks_executor: None,
            max_concurrent_subagents: 3,
            max_children_per_agent: 0,
            max_iterations: None,
            model_resolver: None,
            skill_description_budget: 16000,
            skill_overrides: HashMap::new(),
            tool_profiles: HashMap::new(),
            session_id: None,
            environment: None,
            self_section: None,
            reflection_model: None,
            reflection_config: None,
            event_bus: None,
            model_name: None,
            provider_name: None,
            channel: None,
            agent_id: None,
        }
    }
}

/// Create a deep agent with the given model and options.
///
/// Assembles an interceptor stack and tool set:
/// 1. **EnvironmentMiddleware** — self-awareness injection
/// 2. **SkillsMiddleware** — progressive disclosure of skills + SkillTool
/// 3. **DeepMemoryMiddleware** — loads memory file into system prompt
/// 4. **FilesystemMiddleware** — filesystem tools + large result eviction
/// 5. **SubAgentMiddleware** — `task` tool for child agent spawning
/// 6. **DeepSummarizationMiddleware** — auto-summarize context on overflow
/// 7. **PatchToolCallsMiddleware** — fix malformed tool calls
/// 8. User-provided interceptors
#[traceable(skip = "model,options")]
pub fn create_deep_agent(
    model: Arc<dyn ChatModel>,
    mut options: DeepAgentOptions,
) -> Result<CompiledGraph<MessageState>, SynapticError> {
    let mut all_interceptors: Vec<Arc<dyn Interceptor>> = Vec::new();
    let mut all_tools: Vec<Arc<dyn Tool>> = Vec::new();

    // 0. Environment middleware (highest priority — appears first in system prompt)
    if let Some(env) = options.environment.take() {
        let mut env_mw = middleware::environment::EnvironmentMiddleware::new(env);
        if let Some(self_sec) = options.self_section.clone() {
            env_mw = env_mw.with_self_section(self_sec);
        }
        all_interceptors.push(Arc::new(env_mw));
    }

    // Subagent spawner (created early so SkillTool can reference it)
    let subagent_spawner: Option<Arc<dyn SubAgentSpawner>> = if options.enable_subagents {
        Some(Arc::new(TaskToolSpawner::new(
            options.backend.clone(),
            model.clone(),
            options.max_subagent_depth,
        )))
    } else {
        None
    };

    // 1. Skills middleware + SkillTool (highest priority — loaded first)
    if options.enable_skills && !options.skills_dirs.is_empty() {
        let mut skills_mw = middleware::skills::SkillsMiddleware::with_dirs(
            options.backend.clone(),
            options.skills_dirs.clone(),
            options.command_executor.clone(),
        )
        .with_description_budget(options.skill_description_budget);
        if !options.skill_overrides.is_empty() {
            skills_mw = skills_mw.with_overrides(options.skill_overrides.clone());
        }
        if let Some(ref hooks) = options.hooks_executor {
            skills_mw = skills_mw.with_hooks_executor(hooks.clone());
        }
        let session_id_lock = options
            .session_id
            .as_ref()
            .map(|sid| Arc::new(tokio::sync::RwLock::new(sid.clone())));
        all_tools.push(
            skills_mw.create_skill_tool_with_session(subagent_spawner.clone(), session_id_lock),
        );
        all_interceptors.push(Arc::new(skills_mw));
    }

    // 2. Memory middleware
    if options.enable_memory {
        let memory_file = options
            .memory_file
            .clone()
            .unwrap_or_else(|| "AGENTS.md".to_string());
        all_interceptors.push(Arc::new(middleware::memory::DeepMemoryMiddleware::new(
            options.backend.clone(),
            memory_file,
        )));
    }

    // 3. Filesystem middleware + tools
    if options.enable_filesystem {
        let fs_tools = tools::create_filesystem_tools(options.backend.clone());
        all_tools.extend(fs_tools);
        all_interceptors.push(Arc::new(middleware::filesystem::FilesystemMiddleware::new(
            options.backend.clone(),
            options.eviction_threshold,
        )));
    }

    // 4. Subagent middleware + task tool + TaskOutput tool
    if options.enable_subagents {
        let mut subagent_mw = middleware::subagent::SubAgentMiddleware::with_concurrency(
            options.backend.clone(),
            model.clone(),
            options.max_subagent_depth,
            options.subagents.clone(),
            options.max_concurrent_subagents,
        );
        if let Some(ref resolver) = options.model_resolver {
            subagent_mw = subagent_mw.with_model_resolver(resolver.clone());
        }
        if !options.tool_profiles.is_empty() {
            subagent_mw = subagent_mw.with_tool_profiles(options.tool_profiles.clone());
        }
        if options.max_children_per_agent > 0 {
            subagent_mw = subagent_mw.with_max_children_per_agent(options.max_children_per_agent);
        }
        let bg_registry = subagent_mw.background_registry();
        all_tools.push(subagent_mw.create_task_tool());
        all_tools.push(Arc::new(TaskOutputTool::new(bg_registry)));
    }

    // LlmTaskTool — always available (single-turn lightweight delegation)
    all_tools.push(Arc::new(LlmTaskTool::new(
        model.clone(),
        options.model_resolver.clone(),
    )));

    // 5. Summarization middleware
    all_interceptors.push(Arc::new(
        middleware::summarization::DeepSummarizationMiddleware::new(
            options.backend.clone(),
            model.clone(),
            options.max_input_tokens,
            options.summarization_threshold,
        ),
    ));

    // 6. Patch tool calls middleware
    all_interceptors.push(Arc::new(
        middleware::patch_tool_calls::PatchToolCallsMiddleware,
    ));

    // 7. User-provided interceptors
    all_interceptors.extend(options.interceptors);

    // 8. Reflection subscriber (runs on AgentEnd events via EventBus)
    if let Some(ref reflection_model) = options.reflection_model {
        if let Some(ref bus) = options.event_bus {
            let config = options.reflection_config.clone().unwrap_or_default();
            let reflection = middleware::reflection::ReflectionMiddleware::new(
                reflection_model.clone(),
                options.backend.clone(),
            )
            .with_config(config);
            bus.subscribe(Arc::new(reflection), 100, "reflection");
        } else {
            tracing::warn!(
                "Reflection model configured but no EventBus provided; reflection disabled"
            );
        }
    }

    // Add user-provided tools
    all_tools.extend(options.tools);

    // Build agent options with interceptors
    let agent_options = AgentOptions {
        checkpointer: options.checkpointer,
        interrupt_before: Vec::new(),
        interrupt_after: Vec::new(),
        system_prompt: options.system_prompt,
        interceptors: all_interceptors,
        store: options.store,
        name: Some("deep_agent".to_string()),
        pre_model_hook: None,
        post_model_hook: None,
        response_format: None,
        parallel_tools: options.parallel_tools,
        max_iterations: options.max_iterations,
        event_bus: options.event_bus,
    };

    create_agent(model, all_tools, agent_options)
}
