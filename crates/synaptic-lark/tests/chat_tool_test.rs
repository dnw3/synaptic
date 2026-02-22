use serde_json::json;
use synaptic_core::Tool;
use synaptic_lark::{LarkChatTool, LarkConfig};

// ── Metadata ─────────────────────────────────────────────────────────────────

#[test]
fn chat_tool_metadata() {
    let tool = LarkChatTool::new(LarkConfig::new("cli_test", "secret_test"));
    assert_eq!(tool.name(), "lark_chat");
    assert!(!tool.description().is_empty());
    let params = tool.parameters().expect("should have parameters");
    assert!(params["properties"]["action"].is_object());
    assert!(params["properties"]["chat_id"].is_object());
    assert!(params["properties"]["name"].is_object());
    assert!(params["properties"]["member_open_ids"].is_object());
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("action")));
}

// ── list: no required params ──────────────────────────────────────────────────

#[tokio::test]
async fn list_chats_no_required_params() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let result = tool.call(json!({ "action": "list" })).await;
    // Should fail with network/auth error, NOT a validation error
    let err_str = result.unwrap_err().to_string();
    assert!(!err_str.contains("unknown action"), "got: {err_str}");
    assert!(!err_str.contains("missing"), "got: {err_str}");
}

// ── Validation: get ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_chat_missing_chat_id() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(json!({ "action": "get" })).await.unwrap_err();
    assert!(err.to_string().contains("chat_id"), "got: {err}");
}

// ── Validation: create ────────────────────────────────────────────────────────

#[tokio::test]
async fn create_chat_missing_name() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(json!({ "action": "create" })).await.unwrap_err();
    assert!(err.to_string().contains("name"), "got: {err}");
}

// ── Validation: update ────────────────────────────────────────────────────────

#[tokio::test]
async fn update_chat_missing_chat_id() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "update", "name": "New Name" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("chat_id"), "got: {err}");
}

// ── Validation: list_members ──────────────────────────────────────────────────

#[tokio::test]
async fn list_members_missing_chat_id() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "list_members" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("chat_id"), "got: {err}");
}

// ── Validation: add_members ───────────────────────────────────────────────────

#[tokio::test]
async fn add_members_missing_chat_id() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "add_members", "member_open_ids": ["ou_xxx"] }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("chat_id"), "got: {err}");
}

#[tokio::test]
async fn add_members_missing_open_ids() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "add_members", "chat_id": "oc_xxx" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("member_open_ids"), "got: {err}");
}

// ── Validation: remove_members ────────────────────────────────────────────────

#[tokio::test]
async fn remove_members_missing_chat_id() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "remove_members", "member_open_ids": ["ou_xxx"] }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("chat_id"), "got: {err}");
}

// ── Unknown action ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn chat_unknown_action() {
    let tool = LarkChatTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "dissolve" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown action"), "got: {err}");
}
