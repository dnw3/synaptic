//! Full-request token estimation helpers.
//!
//! These estimate tokens for all components of a model request: messages
//! (including content + tool_calls), system prompt text, and tool definitions
//! (including `input_schema` which is typically the largest component).
//!
//! The old condenser only estimated `content().len() / 4`, missing tool_calls
//! and tool schemas entirely, which caused compaction to never trigger when
//! conversations were tool-call heavy.

use crate::message::Message;
use crate::tool::ToolDefinition;

/// Extra tokens reserved for the summarization prompt injected during compaction.
pub const SUMMARIZATION_OVERHEAD: usize = 4096;

/// Tokens reserved for thinking/reasoning output in extended-thinking models.
pub const THINKING_OUTPUT_RESERVE: usize = 32_000;

/// Estimate the token count for a plain text string.
///
/// Uses the heuristic of ~4 characters per token, plus a fixed per-field
/// overhead of 4 tokens (role marker, separators, etc.).
pub fn estimate_text(text: &str) -> usize {
    text.len() / 4 + 4
}

/// Estimate the token count for a single message, including its content
/// and any tool calls (name + serialized arguments).
pub fn estimate_message_tokens(msg: &Message) -> usize {
    let mut tokens = estimate_text(msg.content());

    for tc in msg.tool_calls() {
        // Tool call name
        tokens += estimate_text(&tc.name);
        // Tool call arguments serialized as JSON
        let args_str = serde_json::to_string(&tc.arguments).unwrap_or_default();
        tokens += estimate_text(&args_str);
    }

    tokens
}

/// Estimate the total token count for a slice of messages.
pub fn estimate_messages(msgs: &[Message]) -> usize {
    msgs.iter().map(estimate_message_tokens).sum()
}

/// Estimate the token count for a set of tool definitions, including their
/// names, descriptions, and `input_schema` (serialized JSON Schema).
pub fn estimate_tools(tools: &[ToolDefinition]) -> usize {
    tools
        .iter()
        .map(|tool| {
            let name_tokens = estimate_text(&tool.name);
            let desc_tokens = estimate_text(&tool.description);
            let schema_str = serde_json::to_string(&tool.input_schema).unwrap_or_default();
            let schema_tokens = estimate_text(&schema_str);
            name_tokens + desc_tokens + schema_tokens
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_text_empty() {
        // Empty string: 0/4 + 4 = 4 (overhead only)
        assert_eq!(estimate_text(""), 4);
    }

    #[test]
    fn test_estimate_text_normal() {
        // "Hello, world!" is 13 chars: 13/4 + 4 = 3 + 4 = 7
        assert_eq!(estimate_text("Hello, world!"), 7);
    }
}
