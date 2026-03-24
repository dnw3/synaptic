use serde_json::json;
use synaptic_core::message::Message;
use synaptic_core::token_estimation::*;
use synaptic_core::tool::{ToolCall, ToolDefinition};

#[test]
fn test_estimate_text_empty() {
    assert_eq!(estimate_text(""), 4);
}

#[test]
fn test_estimate_text_normal() {
    // "Hello, world!" is 13 chars: 13/4 + 4 = 3 + 4 = 7
    assert_eq!(estimate_text("Hello, world!"), 7);
}

#[test]
fn test_estimate_message_with_tool_calls() {
    let msg = Message::ai_with_tool_calls(
        "Let me search for that.",
        vec![ToolCall {
            id: "call_1".into(),
            name: "web_search".into(),
            arguments: json!({"query": "rust programming language"}),
        }],
    );

    let tokens = estimate_message_tokens(&msg);

    // Content: 23 chars -> 23/4 + 4 = 9
    // Tool name "web_search": 10 chars -> 10/4 + 4 = 6
    // Tool args serialized: {"query":"rust programming language"} -> ~38 chars -> 38/4 + 4 = 13
    // Total should be > content-only estimate
    let content_only = estimate_text("Let me search for that.");
    assert!(
        tokens > content_only,
        "message with tool calls ({tokens}) should exceed content-only ({content_only})"
    );
}

#[test]
fn test_estimate_tools_includes_input_schema() {
    let tool = ToolDefinition {
        name: "read_file".into(),
        description: "Read the contents of a file from disk".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["path"]
        }),
        extras: None,
    };

    let tokens = estimate_tools(&[tool]);

    // Should include name + description + full schema serialization
    // The schema alone is substantial, so total should be well above just name+desc
    let name_desc_only =
        estimate_text("read_file") + estimate_text("Read the contents of a file from disk");
    assert!(
        tokens > name_desc_only,
        "tool estimate ({tokens}) should exceed name+desc only ({name_desc_only})"
    );
}

#[test]
fn test_estimate_tools_19_tools() {
    // Production scenario: 19 tools with realistic schemas
    let tools: Vec<ToolDefinition> = (0..19)
        .map(|i| ToolDefinition {
            name: format!("tool_{i}"),
            description: format!("This is tool number {i} which performs an important operation on the system"),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "The primary input parameter for this tool"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "verbose": { "type": "boolean", "description": "Enable verbose output" },
                            "timeout": { "type": "integer", "description": "Timeout in seconds" },
                            "format": {
                                "type": "string",
                                "enum": ["json", "text", "markdown"],
                                "description": "Output format"
                            }
                        }
                    }
                },
                "required": ["input"]
            }),
            extras: None,
        })
        .collect();

    let tokens = estimate_tools(&tools);
    assert!(
        tokens > 500,
        "19 tools with schemas should estimate > 500 tokens, got {tokens}"
    );
}
