//! Skills middleware — multi-directory discovery + system prompt injection.
//!
//! This module is organized into:
//! - `skill_def` — SkillDef struct, frontmatter parsing, loading
//! - `skill_tool` — SkillTool impl, argument substitution
//! - `eligibility` — OS/env/bin filters, is_eligible()

pub mod eligibility;
pub mod skill_def;
pub mod skill_tool;

// Re-export all public types at the module level for backwards compatibility.
pub use eligibility::expand_tilde;
pub use skill_def::{parse_skill_frontmatter, InstallSpec, SkillDef, SkillStatusReport};
pub use skill_tool::{
    resolve_command_placeholders, substitute_arguments, substitute_context_vars, SkillTool,
};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{RunContext, SynapticError, Tool};
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};
use tokio::sync::RwLock;

use crate::backend::Backend;

// ---------------------------------------------------------------------------
// CommandExecutor trait — product layer provides shell execution
// ---------------------------------------------------------------------------

/// Executes `` !`command` `` dynamic placeholders in SKILL.md bodies.
///
/// The framework does not embed shell execution directly; the product layer
/// (e.g. Synapse) provides a concrete implementation.
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, command: &str) -> Result<String, SynapticError>;
}

// ---------------------------------------------------------------------------
// SkillHooksExecutor trait — lifecycle hooks for skills
// ---------------------------------------------------------------------------

/// Lifecycle hook events emitted during skill execution.
#[derive(Debug, Clone)]
pub enum SkillHookEvent {
    /// Fired before a tool call within a skill context.
    PreToolUse {
        skill_name: String,
        tool_name: String,
        tool_input: serde_json::Value,
    },
    /// Fired after a tool call within a skill context.
    PostToolUse {
        skill_name: String,
        tool_name: String,
        tool_input: serde_json::Value,
        tool_output: serde_json::Value,
    },
    /// Fired when the skill completes or is stopped.
    Stop { skill_name: String, reason: String },
}

/// Executes lifecycle hooks defined in SKILL.md `hooks` frontmatter.
///
/// The framework defines the interface; the product layer provides the
/// implementation (e.g. running shell commands, sending notifications).
#[async_trait]
pub trait SkillHooksExecutor: Send + Sync {
    /// Execute a hook. Returns `Ok(true)` to continue, `Ok(false)` to abort.
    async fn execute_hook(&self, event: SkillHookEvent) -> Result<bool, SynapticError>;
}

// ---------------------------------------------------------------------------
// SubAgentSpawner trait — for context: fork integration
// ---------------------------------------------------------------------------

/// Spawns an isolated sub-agent to execute a forked skill.
#[async_trait]
pub trait SubAgentSpawner: Send + Sync {
    async fn spawn(
        &self,
        system_prompt: &str,
        task: &str,
        agent_type: &str,
    ) -> Result<String, SynapticError>;
}

// ---------------------------------------------------------------------------
// SkillOverride — per-skill config overrides
// ---------------------------------------------------------------------------

/// Per-skill configuration overrides, typically loaded from product config.
#[derive(Debug, Clone, Default)]
pub struct SkillOverride {
    /// If Some(false), the skill is disabled entirely.
    pub enabled: Option<bool>,
    /// Environment variable overrides injected into the skill's override_env.
    pub env: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// SkillsMiddleware — multi-directory discovery + system prompt injection
// ---------------------------------------------------------------------------

/// Middleware that discovers skills from multiple directories and injects
/// an index into the system prompt. Also owns the shared skill cache used
/// by [`SkillTool`].
pub struct SkillsMiddleware {
    backend: Arc<dyn Backend>,
    skills_dirs: Vec<String>,
    skills_cache: Arc<RwLock<Vec<SkillDef>>>,
    command_executor: Option<Arc<dyn CommandExecutor>>,
    hooks_executor: Option<Arc<dyn SkillHooksExecutor>>,
    skill_overrides: HashMap<String, SkillOverride>,
    description_budget: usize,
}

impl SkillsMiddleware {
    /// Create with a single skills directory (legacy API).
    pub fn new(backend: Arc<dyn Backend>, skills_dir: String) -> Self {
        Self {
            backend,
            skills_dirs: vec![skills_dir],
            skills_cache: Arc::new(RwLock::new(Vec::new())),
            command_executor: None,
            hooks_executor: None,
            skill_overrides: HashMap::new(),
            description_budget: 16000,
        }
    }

    /// Create with multiple skills directories (higher priority first).
    pub fn with_dirs(
        backend: Arc<dyn Backend>,
        skills_dirs: Vec<String>,
        command_executor: Option<Arc<dyn CommandExecutor>>,
    ) -> Self {
        Self {
            backend,
            skills_dirs,
            skills_cache: Arc::new(RwLock::new(Vec::new())),
            command_executor,
            hooks_executor: None,
            skill_overrides: HashMap::new(),
            description_budget: 16000,
        }
    }

    /// Set the hooks executor for skill lifecycle events.
    pub fn with_hooks_executor(mut self, executor: Arc<dyn SkillHooksExecutor>) -> Self {
        self.hooks_executor = Some(executor);
        self
    }

    /// Set per-skill overrides (enabled/env).
    pub fn with_overrides(mut self, overrides: HashMap<String, SkillOverride>) -> Self {
        self.skill_overrides = overrides;
        self
    }

    /// Get a clone of the shared skills cache for external use.
    pub fn skills_cache(&self) -> Arc<RwLock<Vec<SkillDef>>> {
        self.skills_cache.clone()
    }

    /// Get the skills directories for external watcher setup.
    pub fn skills_dirs(&self) -> &[String] {
        &self.skills_dirs
    }

    /// Force a refresh of the skills cache from disk.
    pub async fn refresh(&self) {
        let fresh = self.discover_skills().await;
        let mut cache = self.skills_cache.write().await;
        *cache = fresh;
    }

    /// Set the token budget for skill descriptions in the system prompt.
    pub fn with_description_budget(mut self, budget: usize) -> Self {
        self.description_budget = budget;
        self
    }

    /// Create the [`SkillTool`] that shares the skill cache.
    pub fn create_skill_tool(
        &self,
        subagent_spawner: Option<Arc<dyn SubAgentSpawner>>,
    ) -> Arc<dyn Tool> {
        self.create_skill_tool_with_session(subagent_spawner, None)
    }

    /// Create the [`SkillTool`] with a shared session ID for context variable substitution.
    pub fn create_skill_tool_with_session(
        &self,
        subagent_spawner: Option<Arc<dyn SubAgentSpawner>>,
        session_id: Option<Arc<RwLock<String>>>,
    ) -> Arc<dyn Tool> {
        Arc::new(SkillTool {
            skills: self.skills_cache.clone(),
            command_executor: self.command_executor.clone(),
            subagent_spawner,
            hooks_executor: self.hooks_executor.clone(),
            session_id: session_id.unwrap_or_else(|| Arc::new(RwLock::new(String::new()))),
        })
    }

    /// Discover skills from all configured directories.
    ///
    /// Higher-priority directories override lower-priority ones by name.
    pub async fn discover_skills(&self) -> Vec<SkillDef> {
        let mut seen = HashSet::new();
        let mut skills = Vec::new();

        for dir in &self.skills_dirs {
            let entries = match self.backend.ls(dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries {
                if !entry.is_dir {
                    // Legacy: flat .md files in commands/ dir
                    if entry.name.ends_with(".md") {
                        let cmd_name = entry.name.trim_end_matches(".md");
                        if seen.contains(cmd_name) {
                            continue;
                        }
                        let file_path = format!("{}/{}", dir, entry.name);
                        if let Ok(content) = self.backend.read_file(&file_path, 0, 500).await {
                            // Try parsing frontmatter; fallback to pure body
                            let mut skill =
                                if let Some(s) = parse_skill_frontmatter(&content, &file_path) {
                                    s
                                } else {
                                    SkillDef {
                                        name: cmd_name.to_string(),
                                        body: content,
                                        path: file_path,
                                        base_dir: dir.clone(),
                                        user_invocable: true,
                                        ..Default::default()
                                    }
                                };
                            // Apply per-skill overrides
                            if let Some(ov) = self.skill_overrides.get(&skill.name) {
                                if ov.enabled == Some(false) {
                                    continue; // skip disabled skill
                                }
                                skill.override_env.extend(ov.env.clone());
                            }
                            seen.insert(skill.name.clone());
                            skills.push(skill);
                        }
                    }
                    continue;
                }

                let skill_path = format!("{}/{}/SKILL.md", dir, entry.name);
                if let Ok(content) = self.backend.read_file(&skill_path, 0, 500).await {
                    if let Some(mut skill) = parse_skill_frontmatter(&content, &skill_path) {
                        if seen.contains(&skill.name) {
                            continue; // higher-priority dir already has this skill
                        }
                        // Apply per-skill overrides
                        if let Some(ov) = self.skill_overrides.get(&skill.name) {
                            if ov.enabled == Some(false) {
                                continue; // skip disabled skill
                            }
                            skill.override_env.extend(ov.env.clone());
                        }
                        seen.insert(skill.name.clone());
                        skills.push(skill);
                    }
                }
            }
        }

        skills
    }
}

#[async_trait]
impl Interceptor for SkillsMiddleware {
    async fn wrap_model_call(
        &self,
        mut request: ModelRequest,
        ctx: &RunContext,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        let skills = self.discover_skills().await;

        // Update shared cache
        {
            let mut cache = self.skills_cache.write().await;
            *cache = skills.clone();
        }

        // Only inject eligible skills that allow model invocation
        let visible: Vec<&SkillDef> = skills
            .iter()
            .filter(|s| !s.disable_model_invocation && s.is_eligible())
            .collect();

        if !visible.is_empty() {
            let mut section = String::from("\n<available_skills>\n");
            let mut budget_used: usize = 0;
            let total = visible.len();

            for (shown, skill) in visible.iter().enumerate() {
                let desc = if skill.description.is_empty() {
                    String::new()
                } else {
                    format!(": {}", skill.description)
                };
                let hint = skill
                    .argument_hint
                    .as_deref()
                    .map(|h| format!(" {}", h))
                    .unwrap_or_default();
                let entry = format!(
                    "- **{}**{}{} (use Skill tool to invoke)\n",
                    skill.name, hint, desc
                );
                let entry_len = entry.len();
                if budget_used + entry_len > self.description_budget && shown > 0 {
                    let remaining = total - shown;
                    section.push_str(&format!("... ({} more skills truncated)\n", remaining));
                    break;
                }
                section.push_str(&entry);
                budget_used += entry_len;
            }
            section.push_str("</available_skills>\n\n");
            section.push_str("To invoke a skill, use the Skill tool with the skill name.\n");
            section.push_str("Users can invoke skills via /skill-name [arguments].\n");

            if let Some(ref mut prompt) = request.system_prompt {
                prompt.push_str(&section);
            } else {
                request.system_prompt = Some(section);
            }
        }

        next.call(request, ctx).await
    }
}
