use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{auth::TokenCache, LarkConfig};

/// Translate text using the Lark machine translation API.
///
/// Supports: `zh`, `en`, `ja`, `ko`, `fr`, `de`, `es`, `pt`, `vi`, `ru`, `hi`, `th`, `it`.
/// The source language is optional — omit it for automatic detection.
pub struct LarkTranslateTool {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl LarkTranslateTool {
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
impl Tool for LarkTranslateTool {
    fn name(&self) -> &'static str {
        "lark_translate"
    }

    fn description(&self) -> &'static str {
        "Translate text using Lark machine translation. \
         Supports zh, en, ja, ko, fr, de, es, pt, vi, ru, hi, th, it."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to translate"
                },
                "source_language": {
                    "type": "string",
                    "description": "Source language code (e.g. zh, en). Omit for auto-detect."
                },
                "target_language": {
                    "type": "string",
                    "description": "Target language code (e.g. en, zh, ja)"
                }
            },
            "required": ["text", "target_language"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let text = args["text"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("lark_translate: missing 'text'".to_string()))?;
        let target = args["target_language"].as_str().ok_or_else(|| {
            SynapticError::Tool("lark_translate: missing 'target_language'".to_string())
        })?;
        let mut body = json!({ "text": text, "target_language": target });
        if let Some(src) = args["source_language"].as_str() {
            body["source_language"] = json!(src);
        }
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/translation/v1/text/translate", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_translate: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_translate parse: {e}")))?;
        if rb["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "lark_translate API error: {}",
                rb["msg"].as_str().unwrap_or("unknown")
            )));
        }
        Ok(json!({ "translated_text": rb["data"]["text"] }))
    }
}
