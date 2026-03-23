use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, Message, SynapticError, Tool};
use synaptic_graph::MessageState;
use tokio::sync::Semaphore;

use crate::backend::Backend;
use crate::middleware::skills::SubAgentSpawner;
use crate::ModelResolver;

use super::registry::BackgroundTaskRegistry;
use super::types::{
    builtin_agent_def, expand_tool_group, filter_tools_by_allow_deny, resolve_agent_memory_dir,
    SubAgentDef,
};

// ---------------------------------------------------------------------------
// TaskTool — full multi-turn sub-agent delegation
// ---------------------------------------------------------------------------

pub(super) struct TaskTool {
    pub(super) backend: Arc<dyn Backend>,
    pub(super) model: Arc<dyn ChatModel>,
    pub(super) max_depth: usize,
    pub(super) current_depth: Arc<AtomicUsize>,
    pub(super) custom_agents: Vec<SubAgentDef>,
    pub(super) concurrency: Arc<Semaphore>,
    pub(super) model_resolver: Option<Arc<dyn ModelResolver>>,
    pub(super) bg_registry: Arc<BackgroundTaskRegistry>,
    pub(super) tool_profiles: HashMap<String, Vec<String>>,
    pub(super) max_children_per_agent: usize,
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
            Ok((response, sub_input_tokens, sub_output_tokens)) => {
                let mut resp = json!({
                    "status": "completed",
                    "result": response,
                    "stats": {
                        "duration_secs": duration.as_secs_f64(),
                        "input_tokens": sub_input_tokens,
                        "output_tokens": sub_output_tokens,
                        "total_tokens": sub_input_tokens + sub_output_tokens,
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
    #[allow(clippy::too_many_arguments)]
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
            options.subagent.enable_subagents = current_depth.load(Ordering::Relaxed) < max_depth;
            options.subagent.max_subagent_depth = max_depth;

            let chosen_model: Arc<dyn ChatModel> = if let Some(om) = model_override {
                if let Some(def) = custom {
                    options.context.system_prompt = Some(def.system_prompt.clone());
                    options.tools =
                        filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
                }
                om
            } else if let Some(def) = custom {
                options.context.system_prompt = Some(def.system_prompt.clone());
                options.tools =
                    filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
                def.model.clone().unwrap_or_else(|| model.clone())
            } else {
                model.clone()
            };

            if let Some(def) = custom {
                if let Some(mt) = def.max_turns {
                    options.condenser.max_iterations = Some(mt);
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
                    // Log background subagent token usage
                    let mut bg_in: u64 = 0;
                    let mut bg_out: u64 = 0;
                    for msg in &final_state.messages {
                        if msg.is_ai() {
                            if let Some(usage) = msg.response_metadata().get("usage") {
                                bg_in += usage["input_tokens"].as_u64().unwrap_or(0);
                                bg_out += usage["output_tokens"].as_u64().unwrap_or(0);
                            }
                        }
                    }
                    tracing::info!(
                        agent_id = %aid,
                        input_tokens = bg_in,
                        output_tokens = bg_out,
                        duration_secs = duration.as_secs_f64(),
                        "background subagent completed"
                    );
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
    ) -> Result<(String, u64, u64), SynapticError> {
        let custom = self.custom_agents.iter().find(|a| a.name == agent_type);
        let builtin = if custom.is_none() {
            builtin_agent_def(agent_type)
        } else {
            None
        };
        let custom = custom.or(builtin.as_ref());

        let mut options = crate::DeepAgentOptions::new(self.backend.clone());
        options.subagent.enable_subagents =
            self.current_depth.load(Ordering::Relaxed) < self.max_depth;
        options.subagent.max_subagent_depth = self.max_depth;

        // Select model: call-arg override > def.model > parent model
        let model: Arc<dyn ChatModel> = if let Some(override_model) = model_override {
            if let Some(def) = custom {
                options.context.system_prompt = Some(def.system_prompt.clone());
                options.tools =
                    filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
            }
            override_model
        } else if let Some(def) = custom {
            options.context.system_prompt = Some(def.system_prompt.clone());
            options.tools = filter_tools_by_allow_deny(&def.tools, &def.tool_allow, &def.tool_deny);
            def.model.clone().unwrap_or_else(|| self.model.clone())
        } else {
            self.model.clone()
        };

        // Apply max_turns if configured
        if let Some(def) = custom {
            if let Some(max_turns) = def.max_turns {
                options.condenser.max_iterations = Some(max_turns);
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
                        let existing = options.context.system_prompt.take().unwrap_or_default();
                        options.context.system_prompt = Some(format!(
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

        // Accumulate subagent token usage from all AI messages
        let mut sub_input: u64 = 0;
        let mut sub_output: u64 = 0;
        for msg in &final_state.messages {
            if msg.is_ai() {
                if let Some(usage) = msg.response_metadata().get("usage") {
                    sub_input += usage["input_tokens"].as_u64().unwrap_or(0);
                    sub_output += usage["output_tokens"].as_u64().unwrap_or(0);
                }
            }
        }

        Ok((response, sub_input, sub_output))
    }
}

// ---------------------------------------------------------------------------
// TaskToolSpawner — SubAgentSpawner implementation
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
        options.context.system_prompt = Some(system_prompt.to_string());
        options.subagent.enable_subagents = false; // forked skills don't get sub-agents
        options.subagent.max_subagent_depth = self.max_depth;

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
