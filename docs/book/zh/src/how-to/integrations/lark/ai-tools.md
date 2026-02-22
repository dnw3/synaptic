# AI 工具

## LarkOcrTool

调用飞书 OCR API（`POST /optical_char_recognition/v1/image/basic_recognize`）从图片中提取文本，适用于在 Agent 中处理截图或扫描件。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkOcrTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkOcrTool::new(config);

// 传入 base64 编码的图片
let result = tool.call(json!({
    "image_base64": "<base64-encoded-image>"
})).await?;

println!("识别文字: {}", result["text"]);
```

---

## LarkTranslateTool

调用飞书翻译 API（`POST /translation/v1/text/translate`）在多种语言之间翻译文本，支持飞书平台提供的所有语言对。

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

println!("翻译结果: {}", result["translated_text"]);
```

---

## LarkAsrTool

调用飞书语音识别 API（`POST /speech_to_text/v1/speech/file_recognize`）将音频文件转录为文本，传入已上传的音频文件的 `file_key`。

```rust,ignore
use synaptic::lark::{LarkAsrTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkAsrTool::new(config);

let result = tool.call(json!({
    "file_key": "file_xxx"
})).await?;

println!("转录文本: {}", result["recognition_text"]);
```

---

## LarkDocProcessTool

调用飞书文档 AI API（`POST /document_ai/v1/entity/recognize`）从文档中提取结构化实体，支持表单、发票等多种文档类型，返回结构化键值对。

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

println!("识别实体: {}", result["entities"]);
```
