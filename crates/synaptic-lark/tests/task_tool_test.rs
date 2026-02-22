use serde_json::json;
use synaptic_core::Tool;
use synaptic_lark::{LarkConfig, LarkTaskTool};

// ── Metadata ─────────────────────────────────────────────────────────────────

#[test]
fn task_tool_metadata() {
    let tool = LarkTaskTool::new(LarkConfig::new("cli_test", "secret_test"));
    assert_eq!(tool.name(), "lark_task");
    assert!(!tool.description().is_empty());
    let params = tool.parameters().expect("should have parameters");
    assert!(params["properties"]["action"].is_object());
    assert!(params["properties"]["task_guid"].is_object());
    assert!(params["properties"]["summary"].is_object());
    assert!(params["properties"]["due_timestamp"].is_object());
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("action")));
}

// ── Validation: get ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_task_missing_guid() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(json!({ "action": "get" })).await.unwrap_err();
    assert!(err.to_string().contains("task_guid"), "got: {err}");
}

// ── Validation: create ────────────────────────────────────────────────────────

#[tokio::test]
async fn create_task_missing_summary() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(json!({ "action": "create" })).await.unwrap_err();
    assert!(err.to_string().contains("summary"), "got: {err}");
}

// ── Validation: update ────────────────────────────────────────────────────────

#[tokio::test]
async fn update_task_missing_guid() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "update", "summary": "New Title" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("task_guid"), "got: {err}");
}

// ── Validation: complete ──────────────────────────────────────────────────────

#[tokio::test]
async fn complete_task_missing_guid() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "complete" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("task_guid"), "got: {err}");
}

// ── Validation: delete ────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_task_missing_guid() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(json!({ "action": "delete" })).await.unwrap_err();
    assert!(err.to_string().contains("task_guid"), "got: {err}");
}

// ── Unknown action ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn task_unknown_action() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(json!({ "action": "archive" })).await.unwrap_err();
    assert!(err.to_string().contains("unknown action"), "got: {err}");
}

// ── Accepted-action smoke tests ───────────────────────────────────────────────

#[tokio::test]
async fn list_tasks_accepted() {
    let tool = LarkTaskTool::new(LarkConfig::new("a", "b"));
    let result = tool.call(json!({ "action": "list" })).await;
    let err_str = result.unwrap_err().to_string();
    assert!(!err_str.contains("unknown action"), "got: {err_str}");
    assert!(!err_str.contains("missing"), "got: {err_str}");
}
