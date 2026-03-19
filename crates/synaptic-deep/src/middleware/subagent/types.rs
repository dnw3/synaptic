use serde_json::Value;
use std::sync::Arc;
use synaptic_core::{ChatModel, Tool};

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

/// Filter tools by allow list (if non-empty) then deny list.
pub fn filter_tools_by_allow_deny(
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

/// Resolve the persistent memory directory for an agent based on scope.
///
/// Scopes: `"user"` → `~/.claude/agent-memory/<name>/`,
///         `"project"` → `.claude/agent-memory/<name>/`,
///         `"local"` → `.claude/agent-memory-local/<name>/`.
pub fn resolve_agent_memory_dir(agent_name: &str, scope: &str) -> std::path::PathBuf {
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
