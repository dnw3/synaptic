# AI Tools

## LarkOcrTool

Extract text from images using the Feishu OCR API (`POST /optical_char_recognition/v1/image/basic_recognize`). Useful for processing screenshots or scanned documents inside an agent.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkOcrTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkOcrTool::new(config);

// Pass a base64-encoded image
let result = tool.call(json!({
    "image_base64": "<base64-encoded-image>"
})).await?;

println!("Recognized text: {}", result["text"]);
```

---

## LarkTranslateTool

Translate text between languages using the Feishu Translation API (`POST /translation/v1/text/translate`). Supports all language pairs offered by the Feishu platform.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkTranslateTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkTranslateTool::new(config);

let result = tool.call(json!({
    "source_language": "zh",
    "target_language": "en",
    "text": "你好，世界！"
})).await?;

println!("Translation: {}", result["translated_text"]);
```

---

## LarkAsrTool

Transcribe audio files to text using the Feishu Speech-to-Text API (`POST /speech_to_text/v1/speech/file_recognize`). Supply a `file_key` of a previously uploaded audio file.

```rust,ignore
use synaptic::lark::{LarkAsrTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkAsrTool::new(config);

let result = tool.call(json!({
    "file_key": "file_xxx"
})).await?;

println!("Transcript: {}", result["recognition_text"]);
```

---

## LarkDocProcessTool

Extract structured entities from documents using the Feishu Document AI API (`POST /document_ai/v1/entity/recognize`). Returns structured key-value pairs from forms, invoices, and other document types.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDocProcessTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkDocProcessTool::new(config);

let result = tool.call(json!({
    "task_type": "invoice",
    "file_key": "file_xxx"
})).await?;

println!("Entities: {}", result["entities"]);
```
