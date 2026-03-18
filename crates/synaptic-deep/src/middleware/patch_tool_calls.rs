#![allow(deprecated)]

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;
use synaptic_core::{Message, SynapticError};
use synaptic_middleware::{AgentMiddleware, ModelRequest, ModelResponse};

/// Middleware that fixes malformed tool calls in model responses.
///
/// Patches applied (after_model):
/// - Strip markdown code fences from JSON arguments
/// - Attempt to parse string arguments as JSON
/// - Deduplicate tool call IDs
/// - Remove tool calls with empty names
///
/// Transcript repair (before_model):
/// - Detect repeated tool-call failures (same tool, missing params) and prune
///   the duplicates to break infinite error loops common with weaker models.
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
pub struct PatchToolCallsMiddleware;

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for PatchToolCallsMiddleware {
    async fn before_model(&self, request: &mut ModelRequest) -> Result<(), SynapticError> {
        prune_repeated_tool_errors(&mut request.messages);
        Ok(())
    }

    async fn after_model(
        &self,
        _request: &ModelRequest,
        response: &mut ModelResponse,
    ) -> Result<(), SynapticError> {
        let tool_calls = response.message.tool_calls().to_vec();
        if tool_calls.is_empty() {
            return Ok(());
        }

        let mut seen_ids = HashSet::new();
        let mut patched = Vec::new();
        let mut id_counter = 0u32;
        let mut changed = false;

        for mut tc in tool_calls {
            // Skip empty names
            if tc.name.trim().is_empty() {
                changed = true;
                continue;
            }

            // Fix JSON arguments
            let fixed_args = fix_json_arguments(&tc.arguments);
            if fixed_args != tc.arguments {
                tc.arguments = fixed_args;
                changed = true;
            }

            // Deduplicate IDs
            if seen_ids.contains(&tc.id) || tc.id.is_empty() {
                tc.id = format!("patched_{}", id_counter);
                id_counter += 1;
                changed = true;
            }
            seen_ids.insert(tc.id.clone());

            patched.push(tc);
        }

        if changed {
            let content = response.message.content().to_string();
            let id = response.message.id().map(|s| s.to_string());
            let mut new_msg = Message::ai_with_tool_calls(content, patched);
            if let Some(id) = id {
                new_msg = new_msg.with_id(id);
            }
            response.message = new_msg;
        }

        Ok(())
    }
}

/// Prune repeated tool-call failures from the transcript.
///
/// When a weak model keeps calling the same tool with missing required params,
/// the transcript accumulates: AI(tool_call) → Tool(error) → AI(tool_call) → Tool(error)...
/// This function detects these cycles and replaces them with a single condensed
/// error + usage hint, breaking the loop.
fn prune_repeated_tool_errors(messages: &mut Vec<Message>) {
    if messages.len() < 4 {
        return;
    }

    // Scan for repeated tool errors: same tool name, same error pattern
    // We look at (assistant + tool_result) pairs from the end
    let mut error_runs: Vec<(String, usize, usize)> = Vec::new(); // (tool_name, start_idx, count)

    let mut i = messages.len();
    while i >= 2 {
        i -= 1;
        let msg = &messages[i];

        // Check if this is a tool result with "missing required parameter" error
        let is_missing_param_error = msg
            .content()
            .to_string()
            .contains("missing required parameter");
        if !is_missing_param_error {
            continue;
        }

        // Check the message before is an AI message with a matching tool call
        if i == 0 {
            break;
        }
        let prev = &messages[i - 1];
        let tool_calls = prev.tool_calls();
        if tool_calls.is_empty() {
            continue;
        }

        let tool_name = tool_calls[0].name.clone();

        // Check if we already have a run for this tool
        if let Some(last_run) = error_runs.last_mut() {
            if last_run.0 == tool_name {
                last_run.1 = i - 1; // extend start
                last_run.2 += 1;
                i -= 1; // skip the AI message too
                continue;
            }
        }

        error_runs.push((tool_name, i - 1, 1));
        i -= 1; // skip the AI message
    }

    // For any tool with 2+ consecutive failures, remove all but the last pair
    // and inject a usage hint into the remaining error message
    for (tool_name, start_idx, count) in error_runs.iter().rev() {
        if *count < 2 {
            continue;
        }

        // Keep only the last (AI + tool_result) pair, remove the rest
        let pairs_to_remove = count - 1;
        let remove_count = pairs_to_remove * 2;

        // Remove from start_idx, remove_count messages
        if *start_idx + remove_count <= messages.len() {
            let hint = format!(
                "\n[HINT: You have called {}() {} times with missing required parameters. \
                 Please check the tool schema carefully and provide ALL required parameters.]",
                tool_name, count
            );

            messages.drain(*start_idx..*start_idx + remove_count);

            // Inject hint into the remaining tool error message
            if *start_idx + 1 < messages.len() {
                let error_msg = &messages[*start_idx + 1];
                let new_content = format!("{}{}", error_msg.content(), hint);
                messages[*start_idx + 1] = Message::tool(
                    error_msg
                        .content()
                        .to_string()
                        .split('\n')
                        .next()
                        .unwrap_or(""),
                    &new_content,
                );
            }
        }
    }
}

fn fix_json_arguments(args: &Value) -> Value {
    if let Value::String(s) = args {
        let trimmed = s.trim();
        // Strip markdown code fences
        let cleaned = if trimmed.starts_with("```") {
            let without_start = trimmed
                .trim_start_matches("```json")
                .trim_start_matches("```");
            without_start.trim_end_matches("```").trim()
        } else {
            trimmed
        };

        // Try to parse as JSON
        match serde_json::from_str(cleaned) {
            Ok(v) => v,
            Err(_) => args.clone(),
        }
    } else {
        args.clone()
    }
}
