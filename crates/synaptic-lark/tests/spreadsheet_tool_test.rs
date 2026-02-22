use serde_json::json;
use synaptic_core::Tool;
use synaptic_lark::{LarkConfig, LarkSpreadsheetTool};

// ── Metadata ─────────────────────────────────────────────────────────────────

#[test]
fn spreadsheet_tool_metadata() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("cli_test", "secret_test"));
    assert_eq!(tool.name(), "lark_spreadsheet");
    assert!(!tool.description().is_empty());
    let params = tool.parameters().expect("should have parameters");
    assert!(params["properties"]["action"].is_object());
    assert!(params["properties"]["spreadsheet_token"].is_object());
    assert!(params["properties"]["range"].is_object());
    assert!(params["properties"]["values"].is_object());
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("action")));
}

// ── Validation: write ─────────────────────────────────────────────────────────

#[tokio::test]
async fn write_missing_token() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "write",
            "range": "Sheet1!A1:B2",
            "values": [["a", "b"]]
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("spreadsheet_token"), "got: {err}");
}

#[tokio::test]
async fn write_missing_range() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "write",
            "spreadsheet_token": "shtcnXxx",
            "values": [["a", "b"]]
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("range"), "got: {err}");
}

#[tokio::test]
async fn write_missing_values() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "write",
            "spreadsheet_token": "shtcnXxx",
            "range": "Sheet1!A1:B2"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("values"), "got: {err}");
}

// ── Validation: append ────────────────────────────────────────────────────────

#[tokio::test]
async fn append_missing_values() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "append",
            "spreadsheet_token": "shtcnXxx",
            "range": "Sheet1!A:B"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("values"), "got: {err}");
}

// ── Validation: clear ─────────────────────────────────────────────────────────

#[tokio::test]
async fn clear_missing_range() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "clear",
            "spreadsheet_token": "shtcnXxx"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("range"), "got: {err}");
}

// ── Validation: read ──────────────────────────────────────────────────────────

#[tokio::test]
async fn read_missing_range() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "read",
            "spreadsheet_token": "shtcnXxx"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("range"), "got: {err}");
}

// ── Unknown action ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn spreadsheet_unknown_action() {
    let tool = LarkSpreadsheetTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "merge_cells" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown action"), "got: {err}");
}
