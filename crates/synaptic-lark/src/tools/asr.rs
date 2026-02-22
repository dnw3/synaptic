use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{auth::TokenCache, LarkConfig};

/// Transcribe a voice/audio file to text using the Lark speech recognition API.
///
/// Accepts a Feishu `file_key` that refers to a previously uploaded audio file.
pub struct LarkAsrTool {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl LarkAsrTool {
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
impl Tool for LarkAsrTool {
    fn name(&self) -> &'static str {
        "lark_asr"
    }

    fn description(&self) -> &'static str {
        "Transcribe a voice/audio file to text using Lark speech recognition. \
         Accepts a Feishu file_key."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "file_key": {
                    "type": "string",
                    "description": "Feishu file_key of the audio file"
                },
                "format": {
                    "type": "string",
                    "description": "Audio format: opus (default), wav, pcm",
                    "enum": ["opus", "wav", "pcm"]
                }
            },
            "required": ["file_key"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let file_key = args["file_key"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("lark_asr: missing 'file_key'".to_string()))?;
        let format = args["format"].as_str().unwrap_or("opus");
        let token = self.token_cache.get_token().await?;
        let file_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();
        let body = json!({
            "speech": { "file_key": file_key },
            "config": { "file_id": file_id, "format": format, "engine_type": "16k_auto" }
        });
        let url = format!("{}/speech_to_text/v1/speech/file_recognize", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_asr: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_asr parse: {e}")))?;
        if rb["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "lark_asr API error: {}",
                rb["msg"].as_str().unwrap_or("unknown")
            )));
        }
        Ok(json!({ "transcript": rb["data"]["recognition_text"] }))
    }
}
