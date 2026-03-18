#![allow(deprecated)]

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use synaptic_core::{SynapticError, Tool};
use synaptic_middleware::{AgentMiddleware, ModelRequest};
use tokio::sync::RwLock;

use crate::backend::Backend;

// ---------------------------------------------------------------------------
// SkillDef — full OpenClaw / Agent Skills Standard
// ---------------------------------------------------------------------------

/// A discovered skill with metadata parsed from YAML frontmatter.
///
/// Compatible with the [Agent Skills Standard](https://agentskills.io/) and
/// the Claude Code / OpenClaw SKILL.md format.
#[derive(Debug, Clone, Default)]
pub struct SkillDef {
    // --- Agent Skills Standard (required) ---
    pub name: String,
    pub description: String,
    /// Full path to the SKILL.md file (relative to backend root).
    pub path: String,
    /// Directory containing the SKILL.md file.
    pub base_dir: String,
    /// Markdown body after the frontmatter.
    pub body: String,

    // --- Agent Skills Standard (optional) ---
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: Option<Value>,

    // --- Claude Code / OpenClaw extensions ---
    /// If true, the model cannot auto-invoke this skill (default: false).
    pub disable_model_invocation: bool,
    /// If true, users can invoke via `/name` (default: true).
    pub user_invocable: bool,
    /// Argument hint shown to users, e.g. "[environment]".
    pub argument_hint: Option<String>,
    /// Tool whitelist for this skill.
    pub allowed_tools: Option<Vec<String>>,
    /// Model override for execution.
    pub model: Option<String>,
    /// Execution context: "fork" spawns an isolated sub-agent.
    pub context: Option<String>,
    /// Agent type when context="fork".
    pub agent: Option<String>,
    /// Lifecycle hooks.
    pub hooks: Option<Value>,
    /// Required environment variables for eligibility.
    pub required_env: Vec<String>,
    /// Required binaries that must be in PATH for eligibility.
    pub required_bins: Vec<String>,

    // --- G-NEW-2: OS filter ---
    /// Operating system filter. Values: "darwin", "linux", "windows".
    /// Empty = all OSes allowed.
    pub os: Vec<String>,

    // --- G-NEW-1: Required config files ---
    /// Config file paths that must exist for eligibility (supports ~ expansion).
    pub required_config: Vec<String>,

    // --- G-NEW-3: Any bins (at least one must be in PATH) ---
    /// At least one of these binaries must be in PATH (vs required_bins which needs all).
    pub required_any_bins: Vec<String>,

    // --- G-NEW-5: Command dispatch ---
    /// If "tool", bypass model and dispatch directly to a tool.
    pub command_dispatch: Option<String>,
    /// Target tool name for command_dispatch = "tool".
    pub command_tool: Option<String>,
    /// Argument mode for command dispatch: "passthrough" or "json".
    pub command_arg_mode: Option<String>,

    // --- G-NEW-6: Per-skill config override env ---
    /// Environment variable overrides injected by SkillOverride.
    pub override_env: HashMap<String, String>,

    // --- Homepage ---
    /// URL for the skill's homepage / documentation.
    pub homepage: Option<String>,

    // --- Always gate ---
    /// If true, this skill is always injected (bypasses eligibility checks).
    pub always: bool,

    // --- Install specs (from metadata.openclaw.install) ---
    pub install: Vec<InstallSpec>,

    // --- Additional metadata fields ---
    pub emoji: Option<String>,
    pub skill_key: Option<String>,
    pub primary_env: Option<String>,
    pub version: Option<String>,
}

/// A dependency installation specification from SKILL.md metadata.
/// Compatible with OpenClaw's install spec format.
#[derive(Debug, Clone, Default)]
pub struct InstallSpec {
    /// Install method: "brew", "node", "go", "uv", "download"
    pub kind: String,
    /// Package/formula/module identifier
    pub package: String,
    /// Expected binaries after installation
    pub bins: Vec<String>,
    /// OS filter for this install spec
    pub os: Vec<String>,
    /// Human-readable label
    pub label: Option<String>,
    /// Unique install spec ID
    pub id: Option<String>,
    /// For brew: use cask instead of formula
    pub cask: bool,
    /// For download: target directory
    pub target_dir: Option<String>,
    /// For download: strip N path components from archive
    pub strip_components: Option<u32>,
}

/// Comprehensive skill status report for UI dashboards and CLI diagnostics.
#[derive(Debug, Clone)]
pub struct SkillStatusReport {
    pub name: String,
    pub description: String,
    pub source: String,
    pub path: String,
    pub eligible: bool,
    pub enabled: bool,
    pub always: bool,
    pub user_invocable: bool,
    pub emoji: Option<String>,
    pub homepage: Option<String>,
    pub skill_key: Option<String>,
    /// Missing environment variables
    pub missing_env: Vec<String>,
    /// Missing binaries (from required_bins)
    pub missing_bins: Vec<String>,
    /// Missing any-bins (none of the alternatives found)
    pub missing_any_bins: Vec<String>,
    /// Missing config files
    pub missing_config: Vec<String>,
    /// OS mismatch (skill requires different OS)
    pub os_mismatch: bool,
    /// Available installation specs
    pub install_specs: Vec<InstallSpec>,
}

impl SkillDef {
    /// Check if this skill's prerequisites are met.
    ///
    /// Returns `true` if all `required_env` vars are set and all
    /// `required_bins` are found in `PATH`.
    pub fn is_eligible(&self) -> bool {
        // Always gate: skip all eligibility checks
        if self.always {
            return true;
        }

        // G-NEW-2: OS filter
        if !self.os.is_empty() && !self.os.iter().any(|o| o == std::env::consts::OS) {
            return false;
        }

        // Required env vars (check override_env first)
        for env_var in &self.required_env {
            if self.override_env.contains_key(env_var) {
                continue; // satisfied by override
            }
            if std::env::var(env_var).is_err() {
                return false;
            }
        }

        // Required bins (all must be in PATH)
        if !self.required_bins.is_empty() {
            let path_var = std::env::var("PATH").unwrap_or_default();
            let paths: Vec<&str> = path_var.split(':').collect();
            for bin in &self.required_bins {
                let found = paths
                    .iter()
                    .any(|p| std::path::Path::new(p).join(bin).exists());
                if !found {
                    return false;
                }
            }
        }

        // G-NEW-3: Any bins (at least one must be in PATH)
        if !self.required_any_bins.is_empty() {
            let path_var = std::env::var("PATH").unwrap_or_default();
            let paths: Vec<&str> = path_var.split(':').collect();
            let any_found = self.required_any_bins.iter().any(|bin| {
                paths
                    .iter()
                    .any(|p| std::path::Path::new(p).join(bin).exists())
            });
            if !any_found {
                return false;
            }
        }

        // G-NEW-1: Required config files
        for config_path in &self.required_config {
            let expanded = expand_tilde(config_path);
            if !std::path::Path::new(&expanded).exists() {
                return false;
            }
        }

        true
    }

    /// Produce a detailed status report showing what's missing and why.
    pub fn diagnose(&self, source: &str, enabled: bool) -> SkillStatusReport {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_var.split(':').collect();

        let missing_env: Vec<String> = self
            .required_env
            .iter()
            .filter(|e| !self.override_env.contains_key(*e) && std::env::var(e).is_err())
            .cloned()
            .collect();

        let missing_bins: Vec<String> = self
            .required_bins
            .iter()
            .filter(|bin| {
                !paths
                    .iter()
                    .any(|p| std::path::Path::new(p).join(bin).exists())
            })
            .cloned()
            .collect();

        let missing_any_bins: Vec<String> = if !self.required_any_bins.is_empty() {
            let any_found = self.required_any_bins.iter().any(|bin| {
                paths
                    .iter()
                    .any(|p| std::path::Path::new(p).join(bin).exists())
            });
            if any_found {
                Vec::new()
            } else {
                self.required_any_bins.clone()
            }
        } else {
            Vec::new()
        };

        let missing_config: Vec<String> = self
            .required_config
            .iter()
            .filter(|c| !std::path::Path::new(&expand_tilde(c)).exists())
            .cloned()
            .collect();

        let os_mismatch = !self.os.is_empty() && !self.os.iter().any(|o| o == std::env::consts::OS);

        SkillStatusReport {
            name: self.name.clone(),
            description: self.description.clone(),
            source: source.to_string(),
            path: self.path.clone(),
            eligible: self.is_eligible(),
            enabled,
            always: self.always,
            user_invocable: self.user_invocable,
            emoji: self.emoji.clone(),
            homepage: self.homepage.clone(),
            skill_key: self.skill_key.clone(),
            missing_env,
            missing_bins,
            missing_any_bins,
            missing_config,
            os_mismatch,
            install_specs: self.install.clone(),
        }
    }
}

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
        tool_input: Value,
    },
    /// Fired after a tool call within a skill context.
    PostToolUse {
        skill_name: String,
        tool_name: String,
        tool_input: Value,
        tool_output: Value,
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
// YAML frontmatter parser
// ---------------------------------------------------------------------------

/// Expand `~` at the start of a path to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

/// Parse install specs from metadata.openclaw.install array.
fn parse_install_specs(metadata: Option<&Value>) -> Vec<InstallSpec> {
    let oc = metadata.and_then(|m| {
        m.get("openclaw")
            .or_else(|| m.get("clawdbot"))
            .or_else(|| m.get("clawdis"))
    });
    let install_arr = match oc.and_then(|o| o.get("install")).and_then(|i| i.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    install_arr
        .iter()
        .filter_map(|spec| {
            let obj = spec.as_object()?;
            let kind = obj.get("kind").and_then(|v| v.as_str())?.to_string();

            // Package field varies by kind
            let package = match kind.as_str() {
                "brew" => obj.get("formula").and_then(|v| v.as_str()),
                "node" => obj.get("package").and_then(|v| v.as_str()),
                "go" => obj.get("module").and_then(|v| v.as_str()),
                "uv" => obj.get("package").and_then(|v| v.as_str()),
                "download" => obj.get("url").and_then(|v| v.as_str()),
                _ => None,
            }?
            .to_string();

            let bins = obj
                .get("bins")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let os = obj
                .get("os")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Some(InstallSpec {
                kind,
                package,
                bins,
                os,
                label: obj.get("label").and_then(|v| v.as_str()).map(String::from),
                id: obj.get("id").and_then(|v| v.as_str()).map(String::from),
                cask: obj.get("cask").and_then(|v| v.as_bool()).unwrap_or(false),
                target_dir: obj
                    .get("targetDir")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                strip_components: obj
                    .get("stripComponents")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
            })
        })
        .collect()
}

/// Extract metadata from the openclaw/clawdbot/clawdis block.
fn get_openclaw_metadata(metadata: Option<&Value>) -> Option<&Value> {
    metadata.and_then(|m| {
        m.get("openclaw")
            .or_else(|| m.get("clawdbot"))
            .or_else(|| m.get("clawdis"))
    })
}

/// Parse YAML frontmatter between `---` markers and the markdown body.
fn parse_skill_frontmatter(content: &str, path: &str) -> Option<SkillDef> {
    let content = content.trim_start_matches('\u{feff}'); // BOM
    let mut lines = content.lines();

    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut fm_lines = Vec::new();
    let mut body = String::new();
    let mut in_body = false;

    for line in lines {
        if !in_body {
            if line.trim() == "---" {
                in_body = true;
                continue;
            }
            fm_lines.push(line);
        } else {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }

    let yaml_str = fm_lines.join("\n");
    let yaml: serde_json::Value = serde_yml::from_str(&yaml_str).ok()?;
    let map = yaml.as_object()?;

    let name = get_str(map, "name")?;

    // Compute base_dir from path
    let base_dir = path
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();

    Some(SkillDef {
        name,
        description: get_str(map, "description").unwrap_or_default(),
        path: path.to_string(),
        base_dir,
        body,

        license: get_str(map, "license"),
        compatibility: get_str(map, "compatibility"),
        metadata: map.get("metadata").cloned(),

        disable_model_invocation: get_bool(map, "disable-model-invocation")
            .or_else(|| get_bool(map, "disable_model_invocation"))
            .unwrap_or(false),
        user_invocable: get_bool(map, "user-invocable")
            .or_else(|| get_bool(map, "user_invocable"))
            .unwrap_or(true),
        argument_hint: get_str(map, "argument-hint").or_else(|| get_str(map, "argument_hint")),
        allowed_tools: parse_allowed_tools(map),
        model: get_str(map, "model"),
        context: get_str(map, "context"),
        agent: get_str(map, "agent"),
        hooks: map.get("hooks").cloned(),
        required_env: get_str_vec(map, "required-env")
            .or_else(|| get_str_vec(map, "required_env"))
            .or_else(|| {
                get_openclaw_metadata(map.get("metadata"))
                    .and_then(|oc| oc.get("requires"))
                    .and_then(|r| r.get("env"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
            })
            .unwrap_or_default(),
        required_bins: get_str_vec(map, "required-bins")
            .or_else(|| get_str_vec(map, "required_bins"))
            .or_else(|| {
                get_openclaw_metadata(map.get("metadata"))
                    .and_then(|oc| oc.get("requires"))
                    .and_then(|r| r.get("bins"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
            })
            .unwrap_or_default(),
        os: get_str_vec(map, "os").unwrap_or_default(),
        required_config: get_str_vec(map, "required-config")
            .or_else(|| get_str_vec(map, "required_config"))
            .unwrap_or_default(),
        required_any_bins: get_str_vec(map, "required-any-bins")
            .or_else(|| get_str_vec(map, "required_any_bins"))
            .or_else(|| {
                get_openclaw_metadata(map.get("metadata"))
                    .and_then(|oc| oc.get("requires"))
                    .and_then(|r| r.get("anyBins"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
            })
            .unwrap_or_default(),
        command_dispatch: get_str(map, "command-dispatch")
            .or_else(|| get_str(map, "command_dispatch")),
        command_tool: get_str(map, "command-tool").or_else(|| get_str(map, "command_tool")),
        command_arg_mode: get_str(map, "command-arg-mode")
            .or_else(|| get_str(map, "command_arg_mode")),
        override_env: HashMap::new(),
        homepage: get_str(map, "homepage"),
        always: get_bool(map, "always").unwrap_or(false),
        install: parse_install_specs(map.get("metadata")),
        emoji: get_openclaw_metadata(map.get("metadata"))
            .and_then(|oc| oc.get("emoji"))
            .and_then(|v| v.as_str())
            .map(String::from),
        skill_key: get_str(map, "skillKey").or_else(|| {
            get_openclaw_metadata(map.get("metadata"))
                .and_then(|oc| oc.get("skillKey"))
                .and_then(|v| v.as_str())
                .map(String::from)
        }),
        primary_env: get_openclaw_metadata(map.get("metadata"))
            .and_then(|oc| oc.get("primaryEnv"))
            .and_then(|v| v.as_str())
            .map(String::from),
        version: get_str(map, "version"),
    })
}

fn get_str(map: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_bool(map: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| v.as_bool())
}

fn get_str_vec(map: &serde_json::Map<String, Value>, key: &str) -> Option<Vec<String>> {
    map.get(key).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
    })
}

/// Parse `allowed-tools` which can be a space-separated string or array.
fn parse_allowed_tools(map: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    let val = map
        .get("allowed-tools")
        .or_else(|| map.get("allowed_tools"))?;

    match val {
        Value::String(s) => Some(s.split_whitespace().map(|t| t.to_string()).collect()),
        Value::Array(arr) => Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        ),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// $ARGUMENTS substitution engine
// ---------------------------------------------------------------------------

/// Substitute `$ARGUMENTS`, `$ARGUMENTS[N]`, and `$0`, `$1`, ... positional placeholders.
///
/// Replacement order: `$ARGUMENTS[N]` → `$ARGUMENTS` → `$N`
/// (bracket syntax must be replaced before the bare `$ARGUMENTS`).
pub fn substitute_arguments(body: &str, arguments: &str) -> String {
    let args: Vec<&str> = if arguments.is_empty() {
        Vec::new()
    } else {
        arguments.split_whitespace().collect()
    };
    let has_placeholder = body.contains("$ARGUMENTS");

    // 1. $ARGUMENTS[N] bracket syntax (must come before bare $ARGUMENTS)
    let mut result = body.to_string();
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("$ARGUMENTS[{}]", i), arg);
    }

    // 2. Bare $ARGUMENTS → full arguments string
    result = result.replace("$ARGUMENTS", arguments);

    // 3. $N positional
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("${}", i), arg);
    }

    if !has_placeholder && !arguments.is_empty() {
        result.push_str(&format!("\n\nARGUMENTS: {}", arguments));
    }
    result
}

/// Substitute `${KEY}` context variables from a map.
///
/// Predefined variables: `CLAUDE_SESSION_ID`, plus any extras the caller provides.
pub fn substitute_context_vars(body: &str, vars: &HashMap<String, String>) -> String {
    let mut result = body.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("${{{}}}", key), value);
    }
    result
}

/// Resolve `` !`command` `` placeholders by executing them via CommandExecutor.
pub async fn resolve_command_placeholders(
    body: &str,
    executor: &dyn CommandExecutor,
) -> Result<String, SynapticError> {
    let mut result = body.to_string();
    // Pattern: !`command`
    while let Some(start) = result.find("!`") {
        let after = start + 2;
        if let Some(end) = result[after..].find('`') {
            let command = &result[after..after + end];
            let output = executor.execute(command).await?;
            result = format!(
                "{}{}{}",
                &result[..start],
                output.trim(),
                &result[after + end + 1..]
            );
        } else {
            break;
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// SkillsMiddleware — multi-directory discovery + system prompt injection
// ---------------------------------------------------------------------------

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

/// Middleware that discovers skills from multiple directories and injects
/// an index into the system prompt. Also owns the shared skill cache used
/// by [`SkillTool`].
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
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

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for SkillsMiddleware {
    async fn before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
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

        if visible.is_empty() {
            return Ok(());
        }

        let mut section = String::from("\n<available_skills>\n");
        let mut budget_used: usize = 0;
        let mut shown = 0usize;
        let total = visible.len();

        for skill in &visible {
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
            shown += 1;
        }
        section.push_str("</available_skills>\n\n");
        section.push_str("To invoke a skill, use the Skill tool with the skill name.\n");
        section.push_str("Users can invoke skills via /skill-name [arguments].\n");

        if let Some(ref mut prompt) = request.system_prompt {
            prompt.push_str(&section);
        } else {
            request.system_prompt = Some(section);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SkillTool — the core invocation mechanism
// ---------------------------------------------------------------------------

/// Tool that loads and returns a skill's content, with argument substitution
/// and dynamic command resolution. Compatible with OpenClaw's `Skill` tool.
pub struct SkillTool {
    skills: Arc<RwLock<Vec<SkillDef>>>,
    command_executor: Option<Arc<dyn CommandExecutor>>,
    subagent_spawner: Option<Arc<dyn SubAgentSpawner>>,
    hooks_executor: Option<Arc<dyn SkillHooksExecutor>>,
    /// Session ID for `${CLAUDE_SESSION_ID}` context variable substitution.
    session_id: Arc<RwLock<String>>,
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "Skill"
    }

    fn description(&self) -> &'static str {
        "Execute a skill within the current conversation. Use this tool when users ask to perform tasks that match available skills, or when a user references a slash command."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "The skill name. E.g., \"commit\", \"review-pr\", or \"pdf\""
                },
                "args": {
                    "type": "string",
                    "description": "Optional arguments for the skill"
                }
            },
            "required": ["skill"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let skill_name = args
            .get("skill")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SynapticError::Tool("missing 'skill' parameter".into()))?;

        let arguments = args.get("args").and_then(|v| v.as_str()).unwrap_or("");

        // Find skill
        let skills = self.skills.read().await;
        let skill = skills
            .iter()
            .find(|s| s.name == skill_name)
            .ok_or_else(|| SynapticError::Tool(format!("skill '{}' not found", skill_name)))?
            .clone();
        drop(skills);

        // G-NEW-5: Command dispatch — bypass model, call tool directly
        if skill.command_dispatch.as_deref() == Some("tool") {
            let tool_name = skill.command_tool.clone().unwrap_or_default();
            // Accept both "raw" (OpenClaw) and "passthrough" (legacy), default to "raw"
            let arg_mode = match skill.command_arg_mode.as_deref() {
                Some("passthrough") | Some("raw") => skill.command_arg_mode.as_deref().unwrap(),
                _ => "raw",
            };
            return Ok(json!({
                "command_dispatch": "tool",
                "command_tool": tool_name,
                "arguments": arguments,
                "arg_mode": arg_mode,
                "skill": skill_name,
            }));
        }

        // Substitute arguments
        let mut body = substitute_arguments(&skill.body, arguments);

        // Substitute context variables (e.g. ${CLAUDE_SESSION_ID}, ${CLAUDE_SKILL_DIR})
        {
            let mut ctx_vars = HashMap::new();
            let sid = self.session_id.read().await;
            if !sid.is_empty() {
                ctx_vars.insert("CLAUDE_SESSION_ID".to_string(), sid.clone());
            }
            if !skill.base_dir.is_empty() {
                ctx_vars.insert("CLAUDE_SKILL_DIR".to_string(), skill.base_dir.clone());
            }
            if !ctx_vars.is_empty() {
                body = substitute_context_vars(&body, &ctx_vars);
            }
        }

        // Resolve !`command` placeholders
        if body.contains("!`") {
            if let Some(ref executor) = self.command_executor {
                body = resolve_command_placeholders(&body, executor.as_ref()).await?;
            }
        }

        // Handle context: fork → spawn sub-agent
        if skill.context.as_deref() == Some("fork") {
            if let Some(ref spawner) = self.subagent_spawner {
                let agent_type = skill.agent.as_deref().unwrap_or("general-purpose");
                let result = spawner.spawn(&body, arguments, agent_type).await?;
                return Ok(json!({
                    "skill": skill_name,
                    "context": "fork",
                    "result": result,
                }));
            }
        }

        let mut response = json!({
            "skill": skill_name,
            "base_path": skill.base_dir,
            "content": body,
        });

        // Include allowed_tools so callers can enforce tool filtering
        if let Some(ref tools) = skill.allowed_tools {
            response["allowed_tools"] = json!(tools);
        }

        // Include model override if specified
        if let Some(ref model) = skill.model {
            response["model"] = json!(model);
        }

        Ok(response)
    }
}
