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

#[cfg(feature = "sandbox")]
pub mod sandbox;

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
pub use middleware::streaming::{StreamingInterceptor, StreamingOutputHandle};
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

/// Filesystem and execution environment options.
#[derive(Default)]
pub struct FilesystemOptions {
    /// Backend for filesystem operations.
    pub backend: Option<Arc<dyn Backend>>,
    /// Enable filesystem tools (default true).
    pub enable_filesystem: bool,
    /// PathGuard for filesystem tool sandbox. If None, a default guard is created from cwd.
    pub path_guard: Option<Arc<crate::tools::path_guard::PathGuard>>,
}

/// Skills middleware configuration.
#[derive(Default)]
pub struct SkillsOptions {
    /// Enable skills middleware (default true).
    pub enable_skills: bool,
    /// Skills directories in the backend, ordered by priority (high first).
    /// Default: `[".claude/skills"]`.
    pub skills_dirs: Vec<String>,
    /// Max chars for skill descriptions in the system prompt (default 16000).
    pub skill_description_budget: usize,
    /// Per-skill overrides (enabled/env).
    pub skill_overrides: HashMap<String, SkillOverride>,
    /// Command executor for resolving !`command` placeholders in skills.
    pub command_executor: Option<Arc<dyn CommandExecutor>>,
    /// Hooks executor for skill lifecycle events.
    pub hooks_executor: Option<Arc<dyn SkillHooksExecutor>>,
}

/// Sub-agent spawning configuration.
#[derive(Default)]
pub struct SubagentOptions {
    /// Enable subagent spawning via task tool (default true).
    pub enable_subagents: bool,
    /// Maximum nested subagent depth (default 3).
    pub max_subagent_depth: usize,
    /// Maximum concurrent sub-agents (default 3).
    pub max_concurrent_subagents: usize,
    /// Maximum concurrent children per agent type (0 = unlimited, default 0).
    pub max_children_per_agent: usize,
    /// Named tool profiles for sub-agents (e.g. "minimal" → ["read_file", "write_file"]).
    pub tool_profiles: HashMap<String, Vec<String>>,
    /// Custom subagent definitions for the task tool.
    pub subagents: Vec<SubAgentDef>,
    /// Resolver for model name aliases (e.g. "sonnet" → model instance).
    pub model_resolver: Option<Arc<dyn ModelResolver>>,
}

/// Context injection and memory options.
#[derive(Default)]
pub struct ContextOptions {
    /// Optional system prompt prepended to all model calls.
    pub system_prompt: Option<String>,
    /// Runtime environment info for self-awareness injection. None = disabled.
    pub environment: Option<EnvironmentInfo>,
    /// Optional product-specific "self" section text.
    pub self_section: Option<String>,
    /// Memory file path in the backend (default "AGENTS.md").
    pub memory_file: Option<String>,
    /// Enable memory middleware (default true).
    pub enable_memory: bool,
    /// Session ID for context variable substitution in skills (e.g. `${CLAUDE_SESSION_ID}`).
    pub session_id: Option<String>,
}

/// Token management and summarization thresholds.
#[derive(Default)]
pub struct CondenserOptions {
    /// Maximum input tokens before summarization (default 128,000).
    pub max_input_tokens: usize,
    /// Fraction of max_input_tokens that triggers summarization (default 0.85).
    pub summarization_threshold: f64,
    /// Token count above which tool results are evicted to files (default 20,000).
    pub eviction_threshold: usize,
    /// Maximum agent iterations / turns (None = default 100).
    pub max_iterations: Option<usize>,
}

/// Observability, events, and reflection.
#[derive(Default)]
pub struct ObservabilityOptions {
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
    /// Optional lightweight model for post-session reflection. None = disabled.
    pub reflection_model: Option<Arc<dyn ChatModel>>,
    /// Reflection configuration. Only used when `reflection_model` is Some.
    pub reflection_config: Option<ReflectionConfig>,
}

/// Configuration for [`create_deep_agent`].
pub struct DeepAgentOptions {
    // Core (ungroupable)
    /// Additional tools beyond the built-in filesystem tools.
    pub tools: Vec<Arc<dyn Tool>>,
    /// Interceptors (model + tool call wrappers).
    pub interceptors: Vec<Arc<dyn Interceptor>>,
    /// Optional checkpointer for graph state persistence.
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    /// Optional store for runtime tool injection.
    pub store: Option<Arc<dyn Store>>,
    /// Enable parallel tool execution in ToolNode (default false).
    pub parallel_tools: bool,

    // Domain groups
    /// Filesystem and execution environment options.
    pub filesystem: FilesystemOptions,
    /// Skills middleware configuration.
    pub skills: SkillsOptions,
    /// Sub-agent spawning configuration.
    pub subagent: SubagentOptions,
    /// Context injection and memory options.
    pub context: ContextOptions,
    /// Token management and summarization thresholds.
    pub condenser: CondenserOptions,
    /// Observability, events, and reflection.
    pub observability: ObservabilityOptions,
}

impl DeepAgentOptions {
    /// Create options with the given backend and sensible defaults.
    pub fn new(backend: Arc<dyn Backend>) -> Self {
        Self {
            tools: Vec::new(),
            interceptors: Vec::new(),
            checkpointer: None,
            store: None,
            parallel_tools: false,
            filesystem: FilesystemOptions {
                backend: Some(backend),
                enable_filesystem: true,
                path_guard: None,
            },
            skills: SkillsOptions {
                enable_skills: true,
                skills_dirs: vec![".claude/skills".to_string()],
                skill_description_budget: 16000,
                skill_overrides: HashMap::new(),
                command_executor: None,
                hooks_executor: None,
            },
            subagent: SubagentOptions {
                enable_subagents: true,
                max_subagent_depth: 3,
                max_concurrent_subagents: 3,
                max_children_per_agent: 0,
                tool_profiles: HashMap::new(),
                subagents: Vec::new(),
                model_resolver: None,
            },
            context: ContextOptions {
                system_prompt: None,
                environment: None,
                self_section: None,
                memory_file: Some("AGENTS.md".to_string()),
                enable_memory: true,
                session_id: None,
            },
            condenser: CondenserOptions {
                max_input_tokens: 128_000,
                summarization_threshold: 0.85,
                eviction_threshold: 20_000,
                max_iterations: None,
            },
            observability: ObservabilityOptions {
                event_bus: None,
                model_name: None,
                provider_name: None,
                channel: None,
                agent_id: None,
                reflection_model: None,
                reflection_config: None,
            },
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

    // Extract backend early — it's required
    let backend = options
        .filesystem
        .backend
        .clone()
        .expect("backend required");

    // 0. Environment middleware (highest priority — appears first in system prompt)
    if let Some(env) = options.context.environment.take() {
        let mut env_mw = middleware::environment::EnvironmentMiddleware::new(env);
        if let Some(self_sec) = options.context.self_section.clone() {
            env_mw = env_mw.with_self_section(self_sec);
        }
        all_interceptors.push(Arc::new(env_mw));
    }

    // Subagent spawner (created early so SkillTool can reference it)
    let subagent_spawner: Option<Arc<dyn SubAgentSpawner>> = if options.subagent.enable_subagents {
        Some(Arc::new(TaskToolSpawner::new(
            backend.clone(),
            model.clone(),
            options.subagent.max_subagent_depth,
        )))
    } else {
        None
    };

    // 1. Skills middleware + SkillTool (highest priority — loaded first)
    if options.skills.enable_skills && !options.skills.skills_dirs.is_empty() {
        let mut skills_mw = middleware::skills::SkillsMiddleware::with_dirs(
            backend.clone(),
            options.skills.skills_dirs.clone(),
            options.skills.command_executor.clone(),
        )
        .with_description_budget(options.skills.skill_description_budget);
        if !options.skills.skill_overrides.is_empty() {
            skills_mw = skills_mw.with_overrides(options.skills.skill_overrides.clone());
        }
        if let Some(ref hooks) = options.skills.hooks_executor {
            skills_mw = skills_mw.with_hooks_executor(hooks.clone());
        }
        let session_id_lock = options
            .context
            .session_id
            .as_ref()
            .map(|sid| Arc::new(tokio::sync::RwLock::new(sid.clone())));
        all_tools.push(
            skills_mw.create_skill_tool_with_session(subagent_spawner.clone(), session_id_lock),
        );
        all_interceptors.push(Arc::new(skills_mw));
    }

    // 2. Memory middleware
    if options.context.enable_memory {
        let memory_file = options
            .context
            .memory_file
            .clone()
            .unwrap_or_else(|| "AGENTS.md".to_string());
        all_interceptors.push(Arc::new(middleware::memory::DeepMemoryMiddleware::new(
            backend.clone(),
            memory_file,
        )));
    }

    // 3. Filesystem middleware + tools
    if options.filesystem.enable_filesystem {
        let path_guard = options.filesystem.path_guard.clone().or_else(|| {
            Some(Arc::new(tools::path_guard::PathGuard::new(
                std::env::current_dir().unwrap_or_default(),
            )))
        });
        let fs_tools = tools::create_filesystem_tools(backend.clone(), path_guard);
        all_tools.extend(fs_tools);
        all_interceptors.push(Arc::new(middleware::filesystem::FilesystemMiddleware::new(
            backend.clone(),
            options.condenser.eviction_threshold,
        )));
    }

    // 4. Subagent middleware + task tool + TaskOutput tool
    if options.subagent.enable_subagents {
        let mut subagent_mw = middleware::subagent::SubAgentMiddleware::with_concurrency(
            backend.clone(),
            model.clone(),
            options.subagent.max_subagent_depth,
            options.subagent.subagents.clone(),
            options.subagent.max_concurrent_subagents,
        );
        if let Some(ref resolver) = options.subagent.model_resolver {
            subagent_mw = subagent_mw.with_model_resolver(resolver.clone());
        }
        if !options.subagent.tool_profiles.is_empty() {
            subagent_mw = subagent_mw.with_tool_profiles(options.subagent.tool_profiles.clone());
        }
        if options.subagent.max_children_per_agent > 0 {
            subagent_mw =
                subagent_mw.with_max_children_per_agent(options.subagent.max_children_per_agent);
        }
        let bg_registry = subagent_mw.background_registry();
        all_tools.push(subagent_mw.create_task_tool());
        all_tools.push(Arc::new(TaskOutputTool::new(bg_registry)));
    }

    // LlmTaskTool — always available (single-turn lightweight delegation)
    all_tools.push(Arc::new(LlmTaskTool::new(
        model.clone(),
        options.subagent.model_resolver.clone(),
    )));

    // 5. Summarization middleware
    all_interceptors.push(Arc::new(
        middleware::summarization::DeepSummarizationMiddleware::new(
            backend.clone(),
            model.clone(),
            options.condenser.max_input_tokens,
            options.condenser.summarization_threshold,
        ),
    ));

    // 6. Patch tool calls middleware
    all_interceptors.push(Arc::new(
        middleware::patch_tool_calls::PatchToolCallsMiddleware,
    ));

    // 7. User-provided interceptors
    all_interceptors.extend(options.interceptors);

    // 8. Reflection subscriber (runs on AgentEnd events via EventBus)
    if let Some(ref reflection_model) = options.observability.reflection_model {
        if let Some(ref bus) = options.observability.event_bus {
            let config = options
                .observability
                .reflection_config
                .clone()
                .unwrap_or_default();
            let reflection = middleware::reflection::ReflectionMiddleware::new(
                reflection_model.clone(),
                backend.clone(),
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
        system_prompt: options.context.system_prompt,
        interceptors: all_interceptors,
        store: options.store,
        name: Some("deep_agent".to_string()),
        pre_model_hook: None,
        post_model_hook: None,
        response_format: None,
        parallel_tools: options.parallel_tools,
        max_iterations: options.condenser.max_iterations,
        event_bus: options.observability.event_bus,
    };

    create_agent(model, all_tools, agent_options)
}
