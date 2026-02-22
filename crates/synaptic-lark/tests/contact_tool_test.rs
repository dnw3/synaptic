use serde_json::json;
use synaptic_core::Tool;
use synaptic_lark::{LarkConfig, LarkContactTool};

// ── Metadata ─────────────────────────────────────────────────────────────────

#[test]
fn contact_tool_metadata() {
    let tool = LarkContactTool::new(LarkConfig::new("cli_test", "secret_test"));
    assert_eq!(tool.name(), "lark_contact");
    assert!(!tool.description().is_empty());
    let params = tool.parameters().expect("should have parameters");
    assert!(params["properties"]["action"].is_object());
    assert!(params["properties"]["user_id"].is_object());
    assert!(params["properties"]["emails"].is_object());
    assert!(params["properties"]["mobiles"].is_object());
    assert!(params["properties"]["department_id"].is_object());
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("action")));
}

// ── Validation: get_user ────────────────────────────────────────────────────

#[tokio::test]
async fn get_user_missing_user_id() {
    let tool = LarkContactTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "get_user" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("user_id"), "got: {err}");
}

// ── Validation: batch_get_id ────────────────────────────────────────────────

#[tokio::test]
async fn batch_get_id_missing_inputs() {
    let tool = LarkContactTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "batch_get_id" }))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("emails") || err.to_string().contains("mobiles"),
        "got: {err}"
    );
}

// ── Validation: get_department ──────────────────────────────────────────────

#[tokio::test]
async fn get_department_missing_id() {
    let tool = LarkContactTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "get_department" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("department_id"), "got: {err}");
}

// ── Unknown action ─────────────────────────────────────────────────────────

#[tokio::test]
async fn contact_unknown_action() {
    let tool = LarkContactTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "delete_user" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown action"), "got: {err}");
}

// ── Accepted-action smoke tests ───────────────────────────────────────────────

#[tokio::test]
async fn list_departments_accepted() {
    let tool = LarkContactTool::new(LarkConfig::new("a", "b"));
    let result = tool.call(json!({ "action": "list_departments" })).await;
    let err_str = result.unwrap_err().to_string();
    assert!(!err_str.contains("unknown action"), "got: {err_str}");
}
