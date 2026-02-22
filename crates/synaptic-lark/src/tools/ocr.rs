use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{auth::TokenCache, LarkConfig};

/// Recognize text in an image using the Lark OCR API.
///
/// Pass the image as a base64-encoded string (`image_base64`) or as a
/// Feishu `file_key` that was obtained from a previous upload.
pub struct LarkOcrTool {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl LarkOcrTool {
    /// Create a new tool using the given config.
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LarkOcrTool {
    fn name(&self) -> &'static str {
        "lark_ocr"
    }

    fn description(&self) -> &'static str {
        "Recognize text in an image using Lark OCR. Provide image as base64 string or a Feishu file_key."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "image_base64": {
                    "type": "string",
                    "description": "Base64-encoded image content (JPEG/PNG)"
                },
                "file_key": {
                    "type": "string",
                    "description": "Feishu file_key of a previously uploaded image"
                }
            }
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let image_b64 = args["image_base64"].as_str();
        let file_key = args["file_key"].as_str();
        if image_b64.is_none() && file_key.is_none() {
            return Err(SynapticError::Tool(
                "lark_ocr: provide image_base64 or file_key".to_string(),
            ));
        }
        let token = self.token_cache.get_token().await?;
        let body = if let Some(b64) = image_b64 {
            json!({ "image": b64 })
        } else {
            json!({ "file_key": file_key.unwrap() })
        };
        let url = format!(
            "{}/optical_char_recognition/v1/image/basic_recognize",
            self.base_url
        );
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_ocr: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_ocr parse: {e}")))?;
        if rb["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "lark_ocr API error: {}",
                rb["msg"].as_str().unwrap_or("unknown")
            )));
        }
        Ok(json!({ "text": rb["data"]["text_list"] }))
    }
}
