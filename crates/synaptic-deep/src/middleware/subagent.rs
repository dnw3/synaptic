#![allow(deprecated)]

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError, Tool};
use synaptic_graph::MessageState;
use synaptic_middleware::AgentMiddleware;
use tokio::sync::{RwLock, Semaphore};

use crate::backend::Backend;
use crate::middleware::skills::SubAgentSpawner;
use crate::ModelResolver;

// ---------------------------------------------------------------------------
// Background task registry — tracks async sub-agent runs
// ---------------------------------------------------------------------------

/// Holds the result of a background sub-agent execution.
#[derive(Debug, Clone)]
pub struct BackgroundTaskResult {
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_secs: Option<f64>,
    /// User-supplied label for this task.
    pub label: Option<String>,
    /// Conversation history preserved for resume support.
    pub messages: Option<Vec<Message>>,
    /// When the task completed (for auto-cleanup).
    pub completed_at: Option<std::time::Instant>,
}

/// Registry for tracking background sub-agent tasks.
#[derive(Default)]
pub struct BackgroundTaskRegistry {
    next_id: AtomicU64,
    tasks: RwLock<HashMap<String, BackgroundTaskResult>>,
    abort_handles: RwLock<HashMap<String, tokio::task::AbortHandle>>,
    /// Tracks running children per agent type for maxChildrenPerAgent.
    active_children: RwLock<HashMap<String, usize>>,
    /// Auto-remove completed tasks after this many seconds (0 = never).
    archive_after_secs: AtomicU64,
}

impl BackgroundTaskRegistry {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            tasks: RwLock::new(HashMap::new()),
            abort_handles: RwLock::new(HashMap::new()),
            active_children: RwLock::new(HashMap::new()),
            archive_after_secs: AtomicU64::new(0),
        }
    }

    /// Set auto-cleanup duration. Completed tasks are removed after this many seconds.
    /// Set to 0 to disable (default).
    pub fn set_archive_after_secs(&self, secs: u64) {
        self.archive_after_secs.store(secs, Ordering::Relaxed);
    }

    /// Remove completed/failed tasks that have exceeded the archive timeout.
    async fn cleanup_archived(&self) {
        let secs = self.archive_after_secs.load(Ordering::Relaxed);
        if secs == 0 {
            return;
        }
        let cutoff = std::time::Duration::from_secs(secs);
        let now = std::time::Instant::now();
        let mut tasks = self.tasks.write().await;
        tasks.retain(|_, t| {
            if let Some(completed_at) = t.completed_at {
                now.duration_since(completed_at) < cutoff
            } else {
                true // keep running tasks
            }
        });
    }

    fn allocate_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("agent-{}", id)
    }

    async fn set_running(&self, id: &str, label: Option<String>) {
        // Opportunistically clean up old completed tasks
        self.cleanup_archived().await;

        let mut tasks = self.tasks.write().await;
        tasks.insert(
            id.to_string(),
            BackgroundTaskResult {
                status: "running".to_string(),
                result: None,
                error: None,
                duration_secs: None,
                label,
                messages: None,
                completed_at: None,
            },
        );
    }

    async fn set_completed(
        &self,
        id: &str,
        result: String,
        duration_secs: f64,
        messages: Option<Vec<Message>>,
    ) {
        let mut tasks = self.tasks.write().await;
        let label = tasks.get(id).and_then(|t| t.label.clone());
        tasks.insert(
            id.to_string(),
            BackgroundTaskResult {
                status: "completed".to_string(),
                result: Some(result),
                error: None,
                duration_secs: Some(duration_secs),
                label,
                messages,
                completed_at: Some(std::time::Instant::now()),
            },
        );
    }

    async fn set_failed(&self, id: &str, error: String) {
        let mut tasks = self.tasks.write().await;
        let label = tasks.get(id).and_then(|t| t.label.clone());
        tasks.insert(
            id.to_string(),
            BackgroundTaskResult {
                status: "failed".to_string(),
                result: None,
                error: Some(error),
                duration_secs: None,
                label,
                messages: None,
                completed_at: Some(std::time::Instant::now()),
            },
        );
    }

    /// Register an abort handle for a background task.
    async fn register_abort_handle(&self, id: &str, handle: tokio::task::AbortHandle) {
        self.abort_handles
            .write()
            .await
            .insert(id.to_string(), handle);
    }

    /// Kill a running background task. Returns true if it was aborted.
    pub async fn kill(&self, id: &str) -> bool {
        if let Some(handle) = self.abort_handles.write().await.remove(id) {
            handle.abort();
            self.set_failed(id, "killed by user".to_string()).await;
            true
        } else {
            false
        }
    }

    /// Get the current status of a background task.
    pub async fn get(&self, id: &str) -> Option<BackgroundTaskResult> {
        self.tasks.read().await.get(id).cloned()
    }

    /// Increment active children count for an agent type.
    pub async fn increment_children(&self, agent_type: &str) {
        let mut map = self.active_children.write().await;
        *map.entry(agent_type.to_string()).or_insert(0) += 1;
    }

    /// Decrement active children count for an agent type.
    pub async fn decrement_children(&self, agent_type: &str) {
        let mut map = self.active_children.write().await;
        if let Some(count) = map.get_mut(agent_type) {
            *count = count.saturating_sub(1);
        }
    }

    /// Get the current active children count for an agent type.
    pub async fn active_children_count(&self, agent_type: &str) -> usize {
        self.active_children
            .read()
            .await
            .get(agent_type)
            .copied()
            .unwrap_or(0)
    }
}

/// Definition of a custom sub-agent type available to the task tool.
#[derive(Clone)]
pub struct SubAgentDef {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<Arc<dyn Tool>>,
    /// Optional model override. `None` = inherit parent model.
    pub model: Option<Arc<dyn ChatModel>>,
    /// Tool names to include (allowlist, supports glob patterns with `*`).
    /// When non-empty, only matching tools are available (before deny filtering).
    pub tool_allow: Vec<String>,
    /// Tool names to exclude (denylist, supports glob patterns with `*`).
    /// Applied after allowlist filtering.
    pub tool_deny: Vec<String>,
    /// Timeout in seconds for the sub-agent execution.
    pub timeout_secs: Option<u64>,
    /// Maximum agentic turns before stopping.
    pub max_turns: Option<usize>,
    /// Named tool profile to apply (filters tools to the profile's list).
    pub tool_profile: Option<String>,
    /// Permission mode: "default", "acceptEdits", "dontAsk", "bypassPermissions", "plan".
    pub permission_mode: Option<String>,
    /// Skill names to preload into the sub-agent's system prompt.
    pub skills: Vec<String>,
    /// If true, the task tool always runs this agent in background mode.
    pub background: bool,
    /// Lifecycle hooks scoped to this agent (same format as SKILL.md hooks).
    pub hooks: Option<Value>,
    /// Persistent memory scope: "user", "project", or "local".
    /// When set, the agent gets a dedicated memory directory and MEMORY.md injection.
    pub memory: Option<String>,
}

/// Returns a built-in agent definition by name, or `None` if unknown.
///
/// Built-in agents: `Explore` (read-only codebase exploration),
/// `Plan` (read-only architecture planning), `Bash` (terminal-focused).
pub fn builtin_agent_def(name: &str) -> Option<SubAgentDef> {
    match name {
        "Explore" | "explore" => Some(SubAgentDef {
            name: "Explore".into(),
            description: "Fast agent for codebase exploration (read-only)".into(),
            system_prompt: "You are a fast exploration agent. Search and read files to answer questions. Do NOT modify any files.".into(),
            tools: vec![],
            model: None,
            tool_allow: vec![],
            tool_deny: vec!["write_file".into(), "edit_file".into(), "delete_file".into(), "apply_patch".into()],
            timeout_secs: Some(120),
            max_turns: Some(20),
            tool_profile: None,
            permission_mode: None,
            skills: vec![],
            background: false,
            hooks: None,
            memory: None,
        }),
        "Plan" | "plan" => Some(SubAgentDef {
            name: "Plan".into(),
            description: "Software architect agent for planning (read-only)".into(),
            system_prompt: "You are a planning agent. Explore the codebase and design implementation approaches. Do NOT modify any files.".into(),
            tools: vec![],
            model: None,
            tool_allow: vec![],
            tool_deny: vec!["write_file".into(), "edit_file".into(), "delete_file".into(), "apply_patch".into()],
            timeout_secs: Some(180),
            max_turns: Some(30),
            tool_profile: None,
            permission_mode: None,
            skills: vec![],
            background: false,
            hooks: None,
            memory: None,
        }),
        "Bash" | "bash" => Some(SubAgentDef {
            name: "Bash".into(),
            description: "Terminal-focused agent for running commands".into(),
            system_prompt: "You are a terminal agent. Execute shell commands to accomplish tasks.".into(),
            tools: vec![],
            model: None,
            tool_allow: vec![],
            tool_deny: vec![],
            timeout_secs: Some(120),
            max_turns: Some(15),
            tool_profile: None,
            permission_mode: None,
            skills: vec![],
            background: false,
            hooks: None,
            memory: None,
        }),
        _ => None,
    }
}

/// Expand a `group:*` reference into a list of tool names.
///
/// Predefined groups:
/// - `group:fs` — filesystem tools
/// - `group:runtime` — execution and shell tools
/// - `group:sessions` — session management tools
/// - `group:memory` — memory / knowledge-base tools
pub fn expand_tool_group(group: &str) -> Vec<String> {
    match group {
        "group:fs" => vec![
            "read_file",
            "write_file",
            "edit_file",
            "delete_file",
            "list_dir",
            "grep",
            "glob",
        ],
        "group:runtime" => vec!["execute", "Bash", "shell_exec"],
        "group:sessions" => vec!["sessions_list", "sessions_history"],
        "group:memory" => vec!["memory_search", "memory_get"],
        _ => vec![],
    }
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

/// Middleware that provides a `task` tool for spawning child agents.
///
/// The `task` tool creates a child deep agent and invokes it with the given description.
/// Recursion is bounded by `max_depth`, concurrency by a semaphore.
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
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

#[allow(deprecated)]
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

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for SubAgentMiddleware {}

// ---------------------------------------------------------------------------

struct TaskTool {
    backend: Arc<dyn Backend>,
    model: Arc<dyn ChatModel>,
    max_depth: usize,
    current_depth: Arc<AtomicUsize>,
    custom_agents: Vec<SubAgentDef>,
    concurrency: Arc<Semaphore>,
    model_resolver: Option<Arc<dyn ModelResolver>>,
    bg_registry: Arc<BackgroundTaskRegistry>,
    tool_profiles: HashMap<String, Vec<String>>,
    max_children_per_agent: usize,
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &'static str {
        "task"
    }

    fn description(&self) -> &'static str {
        "Spawn a sub-agent to handle a complex, multi-step task autonomously"
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A detailed description of the task for the sub-agent"
                },
                "agent_type": {
                    "type": "string",
                    "description": "Type of agent to spawn (default: general-purpose)"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override for this sub-agent"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Run the sub-agent asynchronously and return an agent_id"
                },
                "isolation": {
                    "type": "string",
                    "enum": ["worktree"],
                    "description": "Isolation mode. 'worktree' creates a temporary git worktree."
                },
                "label": {
                    "type": "string",
                    "description": "A short label to identify this task"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for the sub-agent"
                },
                "tool_profile": {
                    "type": "string",
                    "description": "Named tool profile to filter available tools"
                },
                "resume": {
                    "type": "string",
                    "description": "Resume a previous agent by its agent_id, continuing from where it left off"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds for this invocation (overrides agent default)"
                }
            },
            "required": ["description"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let depth = self.current_depth.load(Ordering::Relaxed);
        if depth >= self.max_depth {
            return Err(SynapticError::Tool(format!(
                "max subagent depth ({}) exceeded",
                self.max_depth
            )));
        }

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SynapticError::Tool("missing 'description' parameter".into()))?;

        let agent_type = args
            .get("agent_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general-purpose");

        let run_in_background = args
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let label = args
            .get("label")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tool_profile = args
            .get("tool_profile")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timeout_override = args.get("timeout").and_then(|v| v.as_u64());

        // Resolve model override from args
        let model_override = if let Some(model_name) = args.get("model").and_then(|v| v.as_str()) {
            if let Some(ref resolver) = self.model_resolver {
                Some(resolver.resolve(model_name).await?)
            } else {
                None
            }
        } else {
            None
        };

        // Handle resume: load previous conversation and append new message
        let resume_id = args
            .get("resume")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let resume_messages = if let Some(ref rid) = resume_id {
            let prev = self.bg_registry.get(rid).await.ok_or_else(|| {
                SynapticError::Tool(format!("agent '{}' not found for resume", rid))
            })?;
            prev.messages.clone()
        } else {
            None
        };

        // Check if the agent def forces background mode
        let custom = self.custom_agents.iter().find(|a| a.name == agent_type);
        let builtin = if custom.is_none() {
            builtin_agent_def(agent_type)
        } else {
            None
        };
        let effective_def = custom.or(builtin.as_ref());
        let force_background = effective_def.is_some_and(|d| d.background);

        // Check per-agent-type children limit
        if self.max_children_per_agent > 0 {
            let current = self.bg_registry.active_children_count(agent_type).await;
            if current >= self.max_children_per_agent {
                return Err(SynapticError::Tool(format!(
                    "agent type '{}' has reached the maximum concurrent children limit ({})",
                    agent_type, self.max_children_per_agent
                )));
            }
        }

        if run_in_background || force_background {
            self.bg_registry.increment_children(agent_type).await;
            let result = self
                .run_in_background(
                    description,
                    agent_type,
                    model_override,
                    label,
                    cwd.clone(),
                    tool_profile.clone(),
                    timeout_override,
                )
                .await;
            // Note: decrement happens inside the spawned task when it completes
            return result;
        }

        // Acquire concurrency permit
        let _permit = self
            .concurrency
            .acquire()
            .await
            .map_err(|e| SynapticError::Tool(format!("semaphore error: {}", e)))?;

        self.bg_registry.increment_children(agent_type).await;
        self.current_depth.fetch_add(1, Ordering::Relaxed);
        let start = std::time::Instant::now();
        let result = self
            .run_subagent(
                description,
                agent_type,
                model_override,
                tool_profile,
                resume_messages,
                timeout_override,
            )
            .await;
        let duration = start.elapsed();
        self.current_depth.fetch_sub(1, Ordering::Relaxed);
        self.bg_registry.decrement_children(agent_type).await;

        match result {
            Ok(response) => {
                let mut resp = json!({
                    "status": "completed",
                    "result": response,
                    "stats": {
                        "duration_secs": duration.as_secs_f64(),
                    }
                });
                if let Some(ref l) = label {
                    resp["label"] = json!(l);
                }
                if let Some(ref c) = cwd {
                    resp["cwd"] = json!(c);
                }
                Ok(resp)
            }
            Err(e) => Err(e),
        }
    }
}

impl TaskTool {
    async fn run_in_background(
        &self,
        description: &str,
        agent_type: &str,
        model_override: Option<Arc<dyn ChatModel>>,
        label: Option<String>,
        cwd: Option<String>,
        _tool_profile: Option<String>,
        timeout_override: Option<u64>,
    ) -> Result<Value, SynapticError> {
        let agent_id = self.bg_registry.allocate_id();
        self.bg_registry.set_running(&agent_id, label.clone()).await;

        // Clone everything needed for the spawned task
        let backend = self.backend.clone();
        let model = self.model.clone();
        let max_depth = self.max_depth;
        let current_depth = self.current_depth.clone();
        let custom_agents = self.custom_agents.clone();
        let concurrency = self.concurrency.clone();
        let registry = self.bg_registry.clone();
        let desc = description.to_string();
        let at = agent_type.to_string();
        let aid = agent_id.clone();

        let join_handle = tokio::spawn(async move {
            // Acquire concurrency permit
            let _permit = match concurrency.acquire().await {
                Ok(p) => p,
                Err(e) => {
                    registry
                        .set_failed(&aid, format!("semaphore error: {}", e))
                        .await;
                    registry.decrement_children(&at).await;
                    return;
                }
            };

            current_depth.fetch_add(1, Ordering::Relaxed);
            let start = std::time::Instant::now();

            // Build the agent inline (can't call run_subagent since we don't have &self)
            let builtin_def = builtin_agent_def(&at);
            let custom = custom_agents
                .iter()
                .find(|a| a.name == at)
                .or(builtin_def.as_ref());
            let mut options = crate::DeepAgentOptions::new(backend);
            options.enable_subagents = current_depth.load(Ordering::Relaxed) < max_depth;
            options.max_subagent_depth = max_depth;

            let chosen_model: Arc<dyn ChatModel> = if let Some(om) = model_override {
                if let Some(def) = custom {
                    options.system_prompt = Some(def.system_prompt.clone());
                    options.tools =
                        filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
                }
                om
            } else if let Some(def) = custom {
                options.system_prompt = Some(def.system_prompt.clone());
                options.tools =
                    filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
                def.model.clone().unwrap_or_else(|| model.clone())
            } else {
                model.clone()
            };

            if let Some(def) = custom {
                if let Some(mt) = def.max_turns {
                    options.max_iterations = Some(mt);
                }
            }

            let agent = match crate::create_deep_agent(chosen_model, options) {
                Ok(a) => a,
                Err(e) => {
                    registry.set_failed(&aid, e.to_string()).await;
                    current_depth.fetch_sub(1, Ordering::Relaxed);
                    return;
                }
            };

            let state = MessageState::with_messages(vec![Message::human(&desc)]);
            let timeout = timeout_override
                .or_else(|| custom.and_then(|d| d.timeout_secs))
                .unwrap_or(300);
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(timeout), agent.invoke(state))
                    .await;

            let duration = start.elapsed();
            current_depth.fetch_sub(1, Ordering::Relaxed);

            match result {
                Ok(Ok(graph_result)) => {
                    let final_state = graph_result.into_state();
                    let msgs = final_state.messages.clone();
                    let response = final_state
                        .last_message()
                        .map(|m| m.content().to_string())
                        .unwrap_or_else(|| "completed with no response".to_string());
                    registry
                        .set_completed(&aid, response, duration.as_secs_f64(), Some(msgs))
                        .await;
                }
                Ok(Err(e)) => {
                    registry.set_failed(&aid, e.to_string()).await;
                }
                Err(_) => {
                    registry
                        .set_failed(&aid, format!("timed out after {}s", timeout))
                        .await;
                }
            }

            // Decrement active children count for this agent type
            registry.decrement_children(&at).await;
        });

        // Register abort handle for kill support
        self.bg_registry
            .register_abort_handle(&agent_id, join_handle.abort_handle())
            .await;

        let mut resp = json!({
            "status": "running",
            "agent_id": agent_id,
        });
        if let Some(ref l) = label {
            resp["label"] = json!(l);
        }
        if let Some(ref c) = cwd {
            resp["cwd"] = json!(c);
        }
        Ok(resp)
    }

    async fn run_subagent(
        &self,
        description: &str,
        agent_type: &str,
        model_override: Option<Arc<dyn ChatModel>>,
        tool_profile: Option<String>,
        resume_messages: Option<Vec<Message>>,
        timeout_override: Option<u64>,
    ) -> Result<String, SynapticError> {
        let custom = self.custom_agents.iter().find(|a| a.name == agent_type);
        let builtin = if custom.is_none() {
            builtin_agent_def(agent_type)
        } else {
            None
        };
        let custom = custom.or(builtin.as_ref());

        let mut options = crate::DeepAgentOptions::new(self.backend.clone());
        options.enable_subagents = self.current_depth.load(Ordering::Relaxed) < self.max_depth;
        options.max_subagent_depth = self.max_depth;

        // Select model: call-arg override > def.model > parent model
        let model: Arc<dyn ChatModel> = if let Some(override_model) = model_override {
            if let Some(def) = custom {
                options.system_prompt = Some(def.system_prompt.clone());
                options.tools =
                    filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
            }
            override_model
        } else if let Some(def) = custom {
            options.system_prompt = Some(def.system_prompt.clone());
            options.tools = filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
            def.model.clone().unwrap_or_else(|| self.model.clone())
        } else {
            self.model.clone()
        };

        // Apply max_turns if configured
        if let Some(def) = custom {
            if let Some(max_turns) = def.max_turns {
                options.max_iterations = Some(max_turns);
            }
        }

        // Apply tool profile filtering
        let effective_profile =
            tool_profile.or_else(|| custom.and_then(|d| d.tool_profile.clone()));
        if let Some(profile_name) = effective_profile {
            // Support group: references (expand into tool names)
            if profile_name.starts_with("group:") {
                let expanded = expand_tool_group(&profile_name);
                if !expanded.is_empty() {
                    options
                        .tools
                        .retain(|t| expanded.contains(&t.name().to_string()));
                }
            } else if let Some(allowed) = self.tool_profiles.get(&profile_name) {
                // Expand any group: entries within the profile
                let mut expanded: Vec<String> = Vec::new();
                for entry in allowed {
                    if entry.starts_with("group:") {
                        expanded.extend(expand_tool_group(entry));
                    } else {
                        expanded.push(entry.clone());
                    }
                }
                options
                    .tools
                    .retain(|t| expanded.contains(&t.name().to_string()));
            }
        }

        // Inject agent persistent memory (MEMORY.md) into system prompt
        if let Some(def) = custom {
            if let Some(ref scope) = def.memory {
                let memory_dir = resolve_agent_memory_dir(&def.name, scope);
                let memory_md = memory_dir.join("MEMORY.md");
                if memory_md.exists() {
                    if let Ok(content) = std::fs::read_to_string(&memory_md) {
                        // Truncate to first 200 lines like CLAUDE.md convention
                        let truncated: String =
                            content.lines().take(200).collect::<Vec<_>>().join("\n");
                        let existing = options.system_prompt.take().unwrap_or_default();
                        options.system_prompt = Some(format!(
                            "{}\n\n# Agent Memory\n\nPersistent memory from {}:\n\n{}",
                            existing,
                            memory_md.display(),
                            truncated
                        ));
                    }
                }
                // Ensure the memory directory exists for the agent to write to
                let _ = std::fs::create_dir_all(&memory_dir);
            }
        }

        let agent = crate::create_deep_agent(model, options)?;

        // Build initial state: resume from previous messages or start fresh
        let state = if let Some(mut prev_msgs) = resume_messages {
            prev_msgs.push(Message::human(description));
            MessageState::with_messages(prev_msgs)
        } else {
            MessageState::with_messages(vec![Message::human(description)])
        };

        // Apply timeout: per-call override > agent def > default 300s
        let timeout_secs = timeout_override
            .or_else(|| custom.and_then(|d| d.timeout_secs))
            .unwrap_or(300);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            agent.invoke(state),
        )
        .await
        .map_err(|_| {
            SynapticError::Tool(format!(
                "sub-agent timed out after {} seconds",
                timeout_secs
            ))
        })?;

        let final_state = result?.into_state();
        let response = final_state
            .last_message()
            .map(|m| m.content().to_string())
            .unwrap_or_else(|| "Sub-agent completed with no response".to_string());

        Ok(response)
    }
}

/// Filter tools by allow list (if non-empty) then deny list.
fn filter_tools_by_allow_deny(
    tools: &[Arc<dyn Tool>],
    allow: &[String],
    deny: &[String],
) -> Vec<Arc<dyn Tool>> {
    let mut filtered: Vec<Arc<dyn Tool>> = if allow.is_empty() {
        tools.to_vec()
    } else {
        tools
            .iter()
            .filter(|t| is_tool_matched(t.name(), allow))
            .cloned()
            .collect()
    };
    if !deny.is_empty() {
        filtered.retain(|t| !is_tool_denied(t.name(), deny));
    }
    filtered
}

/// Check if a tool name matches any pattern in a list (for allowlist).
fn is_tool_matched(tool_name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern == "*" {
            true
        } else if pattern.contains('*') {
            let prefix = pattern.trim_end_matches('*');
            tool_name.starts_with(prefix)
        } else {
            tool_name == pattern
        }
    })
}

/// Resolve the persistent memory directory for an agent based on scope.
///
/// Scopes: `"user"` → `~/.claude/agent-memory/<name>/`,
///         `"project"` → `.claude/agent-memory/<name>/`,
///         `"local"` → `.claude/agent-memory-local/<name>/`.
fn resolve_agent_memory_dir(agent_name: &str, scope: &str) -> std::path::PathBuf {
    match scope {
        "user" => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            std::path::PathBuf::from(home)
                .join(".claude/agent-memory")
                .join(agent_name)
        }
        "local" => std::path::PathBuf::from(".claude/agent-memory-local").join(agent_name),
        _ => {
            // "project" or default
            std::path::PathBuf::from(".claude/agent-memory").join(agent_name)
        }
    }
}

/// Check if a tool name matches any deny pattern.
fn is_tool_denied(tool_name: &str, deny_list: &[String]) -> bool {
    deny_list.iter().any(|pattern| {
        if pattern == "*" {
            true
        } else if pattern.contains('*') {
            let prefix = pattern.trim_end_matches('*');
            tool_name.starts_with(prefix)
        } else {
            tool_name == pattern
        }
    })
}

// ---------------------------------------------------------------------------
// SubAgentSpawner implementation for TaskTool
// ---------------------------------------------------------------------------

/// A spawner backed by the TaskTool's infrastructure.
pub struct TaskToolSpawner {
    backend: Arc<dyn Backend>,
    model: Arc<dyn ChatModel>,
    max_depth: usize,
}

impl TaskToolSpawner {
    pub fn new(backend: Arc<dyn Backend>, model: Arc<dyn ChatModel>, max_depth: usize) -> Self {
        Self {
            backend,
            model,
            max_depth,
        }
    }
}

#[async_trait]
impl SubAgentSpawner for TaskToolSpawner {
    async fn spawn(
        &self,
        system_prompt: &str,
        task: &str,
        _agent_type: &str,
    ) -> Result<String, SynapticError> {
        let mut options = crate::DeepAgentOptions::new(self.backend.clone());
        options.system_prompt = Some(system_prompt.to_string());
        options.enable_subagents = false; // forked skills don't get sub-agents
        options.max_subagent_depth = self.max_depth;

        let agent = crate::create_deep_agent(self.model.clone(), options)?;
        let state = MessageState::with_messages(vec![Message::human(task)]);

        let result = tokio::time::timeout(std::time::Duration::from_secs(300), agent.invoke(state))
            .await
            .map_err(|_| SynapticError::Tool("forked skill timed out after 300s".into()))?;

        let final_state = result?.into_state();
        Ok(final_state
            .last_message()
            .map(|m| m.content().to_string())
            .unwrap_or_else(|| "Forked skill completed with no response".to_string()))
    }
}

// ---------------------------------------------------------------------------
// TaskOutputTool — check status of background sub-agents
// ---------------------------------------------------------------------------

/// Tool for retrieving the output of a background sub-agent task.
pub struct TaskOutputTool {
    registry: Arc<BackgroundTaskRegistry>,
}

impl TaskOutputTool {
    pub fn new(registry: Arc<BackgroundTaskRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &'static str {
        "TaskOutput"
    }

    fn description(&self) -> &'static str {
        "Retrieve the output from a background sub-agent task by its agent_id"
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The agent_id returned when the task was started"
                },
                "action": {
                    "type": "string",
                    "enum": ["status", "kill"],
                    "description": "Action to perform: 'status' (default) or 'kill' to abort the task"
                }
            },
            "required": ["task_id"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let task_id = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SynapticError::Tool("missing 'task_id' parameter".into()))?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("status");

        if action == "kill" {
            let killed = self.registry.kill(task_id).await;
            return Ok(json!({
                "task_id": task_id,
                "action": "kill",
                "killed": killed,
            }));
        }

        let result = self
            .registry
            .get(task_id)
            .await
            .ok_or_else(|| SynapticError::Tool(format!("task '{}' not found", task_id)))?;

        Ok(json!({
            "task_id": task_id,
            "status": result.status,
            "result": result.result,
            "error": result.error,
            "duration_secs": result.duration_secs,
            "label": result.label,
        }))
    }
}

// ---------------------------------------------------------------------------
// LlmTaskTool — single-turn lightweight delegation
// ---------------------------------------------------------------------------

/// A lightweight tool that makes a single model.chat() call without tool loops.
///
/// Unlike the full `task` tool which spawns a deep agent with multi-turn capabilities,
/// this tool does one model call and returns the response. Useful for quick tasks
/// like summarization, translation, or formatting.
pub struct LlmTaskTool {
    model: Arc<dyn ChatModel>,
    model_resolver: Option<Arc<dyn ModelResolver>>,
}

impl LlmTaskTool {
    pub fn new(model: Arc<dyn ChatModel>, model_resolver: Option<Arc<dyn ModelResolver>>) -> Self {
        Self {
            model,
            model_resolver,
        }
    }
}

#[async_trait]
impl Tool for LlmTaskTool {
    fn name(&self) -> &'static str {
        "llm_task"
    }

    fn description(&self) -> &'static str {
        "Run a single LLM call without tool loops. Useful for quick delegation tasks like summarization, translation, or formatting."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt/question to send to the model"
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt for the model"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model name override"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens in the response (default: 4096)"
                }
            },
            "required": ["prompt"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SynapticError::Tool("missing 'prompt' parameter".into()))?;

        let system_prompt = args.get("system_prompt").and_then(|v| v.as_str());

        let max_tokens = args
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096) as u32;

        // Resolve model
        let model = if let Some(model_name) = args.get("model").and_then(|v| v.as_str()) {
            if let Some(ref resolver) = self.model_resolver {
                resolver.resolve(model_name).await?
            } else {
                self.model.clone()
            }
        } else {
            self.model.clone()
        };

        // Build messages
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(Message::system(sys));
        }
        messages.push(Message::human(prompt));

        let _ = max_tokens; // reserved for future use with ChatRequest extensions

        // Single model call
        let request = ChatRequest::new(messages);
        let response = model
            .chat(request)
            .await
            .map_err(|e| SynapticError::Tool(format!("llm_task model error: {}", e)))?;

        let content = response.message.content();
        Ok(json!({
            "response": content,
        }))
    }
}
