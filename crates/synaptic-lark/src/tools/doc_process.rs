use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{auth::TokenCache, LarkConfig};

/// Extract structured data from documents using the Lark intelligent document processing API.
///
/// Supports invoice, receipt, ID card, and general document extraction.
pub struct LarkDocProcessTool {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl LarkDocProcessTool {
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
impl Tool for LarkDocProcessTool {
    fn name(&self) -> &'static str {
        "lark_doc_process"
    }

    fn description(&self) -> &'static str {
        "Extract structured data from documents (PDF, images) using Lark intelligent document \
         processing. Supports invoice, receipt, ID card, and general document extraction."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "file_key": {
                    "type": "string",
                    "description": "Feishu file_key of the document"
                },
                "task_type": {
                    "type": "string",
                    "description": "Extraction type: invoice | receipt | id_card | general",
                    "enum": ["invoice", "receipt", "id_card", "general"]
                }
            },
            "required": ["file_key", "task_type"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let file_key = args["file_key"].as_str().ok_or_else(|| {
            SynapticError::Tool("lark_doc_process: missing 'file_key'".to_string())
        })?;
        let task_type = args["task_type"].as_str().ok_or_else(|| {
            SynapticError::Tool("lark_doc_process: missing 'task_type'".to_string())
        })?;
        let token = self.token_cache.get_token().await?;
        let body = json!({ "file_key": file_key, "task_type": task_type });
        let url = format!("{}/document_ai/v1/entity/recognize", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_doc_process: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("lark_doc_process parse: {e}")))?;
        if rb["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "lark_doc_process API error: {}",
                rb["msg"].as_str().unwrap_or("unknown")
            )));
        }
        Ok(rb["data"].clone())
    }
}
