use serde_json::json;
use synapse_core::{Message, ToolCall};

#[test]
fn system_message_factory() {
    let msg = Message::system("You are helpful");
    assert_eq!(msg.content(), "You are helpful");
    assert_eq!(msg.role(), "system");
    assert!(msg.is_system());
    assert!(!msg.is_human());
}

#[test]
fn human_message_factory() {
    let msg = Message::human("Hello");
    assert_eq!(msg.content(), "Hello");
    assert_eq!(msg.role(), "human");
    assert!(msg.is_human());
}

#[test]
fn ai_message_factory() {
    let msg = Message::ai("I can help");
    assert_eq!(msg.content(), "I can help");
    assert_eq!(msg.role(), "assistant");
    assert!(msg.is_ai());
    assert!(msg.tool_calls().is_empty());
}

#[test]
fn ai_message_with_tool_calls() {
    let msg = Message::ai_with_tool_calls(
        "calling tool",
        vec![ToolCall {
            id: "call-1".into(),
            name: "search".into(),
            arguments: json!({"q": "rust"}),
        }],
    );
    assert_eq!(msg.tool_calls().len(), 1);
    assert_eq!(msg.tool_calls()[0].name, "search");
}

#[test]
fn tool_message_factory() {
    let msg = Message::tool("result data", "call-1");
    assert_eq!(msg.content(), "result data");
    assert_eq!(msg.role(), "tool");
    assert!(msg.is_tool());
    assert_eq!(msg.tool_call_id(), Some("call-1"));
}

#[test]
fn tool_call_id_none_for_non_tool() {
    let msg = Message::human("hi");
    assert_eq!(msg.tool_call_id(), None);
}

#[test]
fn message_serde_roundtrip() {
    let msg = Message::ai_with_tool_calls(
        "using tool",
        vec![ToolCall {
            id: "c1".into(),
            name: "calc".into(),
            arguments: json!({"x": 1}),
        }],
    );
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn message_serde_system_format() {
    let msg = Message::system("be helpful");
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["role"], "system");
    assert_eq!(json["content"], "be helpful");
}

#[test]
fn message_serde_tool_calls_omitted_when_empty() {
    let msg = Message::ai("hello");
    let json = serde_json::to_value(&msg).unwrap();
    assert!(json.get("tool_calls").is_none());
}
