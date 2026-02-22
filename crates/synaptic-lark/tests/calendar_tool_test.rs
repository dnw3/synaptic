use serde_json::json;
use synaptic_core::Tool;
use synaptic_lark::{LarkCalendarTool, LarkConfig};

// ── Metadata ─────────────────────────────────────────────────────────────────

#[test]
fn calendar_tool_metadata() {
    let tool = LarkCalendarTool::new(LarkConfig::new("cli_test", "secret_test"));
    assert_eq!(tool.name(), "lark_calendar");
    assert!(!tool.description().is_empty());
    let params = tool.parameters().expect("should have parameters");
    assert!(params["properties"]["action"].is_object());
    assert!(params["properties"]["calendar_id"].is_object());
    assert!(params["properties"]["event_id"].is_object());
    assert!(params["properties"]["summary"].is_object());
    assert!(params["properties"]["start_time"].is_object());
    assert!(params["properties"]["end_time"].is_object());
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("action")));
}

// ── Validation: list_events ───────────────────────────────────────────────────

#[tokio::test]
async fn list_events_missing_calendar_id() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "list_events" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("calendar_id"), "got: {err}");
}

// ── Validation: get_event ─────────────────────────────────────────────────────

#[tokio::test]
async fn get_event_missing_event_id() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "get_event", "calendar_id": "cal_xxx" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("event_id"), "got: {err}");
}

// ── Validation: create_event ──────────────────────────────────────────────────

#[tokio::test]
async fn create_event_missing_summary() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "create_event",
            "calendar_id": "cal_xxx",
            "start_time": "1735689600",
            "end_time": "1735693200"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("summary"), "got: {err}");
}

#[tokio::test]
async fn create_event_missing_start_time() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "create_event",
            "calendar_id": "cal_xxx",
            "summary": "Team Sync",
            "end_time": "1735693200"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("start_time"), "got: {err}");
}

#[tokio::test]
async fn create_event_missing_end_time() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "create_event",
            "calendar_id": "cal_xxx",
            "summary": "Team Sync",
            "start_time": "1735689600"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("end_time"), "got: {err}");
}

// ── Validation: update_event ──────────────────────────────────────────────────

#[tokio::test]
async fn update_event_missing_calendar_id() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "update_event",
            "event_id": "evt_xxx",
            "summary": "New Title"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("calendar_id"), "got: {err}");
}

// ── Validation: delete_event ──────────────────────────────────────────────────

#[tokio::test]
async fn delete_event_missing_event_id() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({
            "action": "delete_event",
            "calendar_id": "cal_xxx"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("event_id"), "got: {err}");
}

// ── Unknown action ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn calendar_unknown_action() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(json!({ "action": "import_ical" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown action"), "got: {err}");
}

// ── Accepted-action smoke tests ───────────────────────────────────────────────

#[tokio::test]
async fn list_calendars_accepted() {
    let tool = LarkCalendarTool::new(LarkConfig::new("a", "b"));
    let result = tool.call(json!({ "action": "list_calendars" })).await;
    let err_str = result.unwrap_err().to_string();
    assert!(!err_str.contains("unknown action"), "got: {err_str}");
    assert!(!err_str.contains("missing"), "got: {err_str}");
}
