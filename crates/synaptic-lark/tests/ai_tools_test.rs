use synaptic_core::Tool;
use synaptic_lark::{LarkAsrTool, LarkConfig, LarkDocProcessTool, LarkOcrTool, LarkTranslateTool};

#[test]
fn ocr_metadata() {
    let tool = LarkOcrTool::new(LarkConfig::new("a", "b"));
    assert_eq!(tool.name(), "lark_ocr");
    let p = tool.parameters().unwrap();
    assert!(p["properties"]["image_base64"].is_object() || p["properties"]["file_key"].is_object());
}

#[test]
fn translate_metadata() {
    let tool = LarkTranslateTool::new(LarkConfig::new("a", "b"));
    assert_eq!(tool.name(), "lark_translate");
    let p = tool.parameters().unwrap();
    assert!(p["properties"]["text"].is_object());
    assert!(p["properties"]["source_language"].is_object());
    assert!(p["properties"]["target_language"].is_object());
}

#[test]
fn asr_metadata() {
    let tool = LarkAsrTool::new(LarkConfig::new("a", "b"));
    assert_eq!(tool.name(), "lark_asr");
    let p = tool.parameters().unwrap();
    assert!(p["properties"]["file_key"].is_object());
}

#[test]
fn doc_process_metadata() {
    let tool = LarkDocProcessTool::new(LarkConfig::new("a", "b"));
    assert_eq!(tool.name(), "lark_doc_process");
    let p = tool.parameters().unwrap();
    assert!(p["properties"]["file_key"].is_object());
    assert!(p["properties"]["task_type"].is_object());
}

#[tokio::test]
async fn translate_missing_text() {
    let tool = LarkTranslateTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(serde_json::json!({"target_language": "en"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("text"));
}

#[tokio::test]
async fn translate_missing_target() {
    let tool = LarkTranslateTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(serde_json::json!({"text": "hello"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("target_language"));
}

#[tokio::test]
async fn ocr_missing_both_inputs() {
    let tool = LarkOcrTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(serde_json::json!({})).await.unwrap_err();
    assert!(
        err.to_string().contains("image_base64") || err.to_string().contains("file_key"),
        "got: {err}"
    );
}

#[tokio::test]
async fn asr_missing_file_key() {
    let tool = LarkAsrTool::new(LarkConfig::new("a", "b"));
    let err = tool.call(serde_json::json!({})).await.unwrap_err();
    assert!(err.to_string().contains("file_key"), "got: {err}");
}

#[tokio::test]
async fn doc_process_missing_file_key() {
    let tool = LarkDocProcessTool::new(LarkConfig::new("a", "b"));
    let err = tool
        .call(serde_json::json!({"task_type": "invoice"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("file_key"), "got: {err}");
}
