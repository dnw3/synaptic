//! SkillTool — the core invocation mechanism for skills.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};
use tokio::sync::RwLock;

use super::skill_def::SkillDef;
use super::{CommandExecutor, SkillHooksExecutor, SubAgentSpawner};

// ---------------------------------------------------------------------------
// $ARGUMENTS substitution engine
// ---------------------------------------------------------------------------

/// Substitute `$ARGUMENTS`, `$ARGUMENTS[N]`, and `$0`, `$1`, ... positional placeholders.
///
/// Replacement order: `$ARGUMENTS[N]` -> `$ARGUMENTS` -> `$N`
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

    // 2. Bare $ARGUMENTS -> full arguments string
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
// SkillTool
// ---------------------------------------------------------------------------

/// Tool that loads and returns a skill's content, with argument substitution
/// and dynamic command resolution. Compatible with OpenClaw's `Skill` tool.
pub struct SkillTool {
    pub(crate) skills: Arc<RwLock<Vec<SkillDef>>>,
    pub(crate) command_executor: Option<Arc<dyn CommandExecutor>>,
    pub(crate) subagent_spawner: Option<Arc<dyn SubAgentSpawner>>,
    #[allow(dead_code)]
    pub(crate) hooks_executor: Option<Arc<dyn SkillHooksExecutor>>,
    /// Session ID for `${CLAUDE_SESSION_ID}` context variable substitution.
    pub(crate) session_id: Arc<RwLock<String>>,
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

        // Handle context: fork -> spawn sub-agent
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
