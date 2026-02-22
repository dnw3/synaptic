use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{MemoryStore, Message, SynapticError};

use crate::{auth::TokenCache, LarkConfig};

pub struct LarkBitableMemoryStore {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
    app_token: String,
    table_id: String,
}

impl LarkBitableMemoryStore {
    pub fn new(
        config: LarkConfig,
        app_token: impl Into<String>,
        table_id: impl Into<String>,
    ) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
            app_token: app_token.into(),
            table_id: table_id.into(),
        }
    }

    pub fn app_token(&self) -> &str {
        &self.app_token
    }

    pub fn table_id(&self) -> &str {
        &self.table_id
    }

    fn records_url(&self) -> String {
        format!(
            "{}/bitable/v1/apps/{}/tables/{}/records",
            self.base_url, self.app_token, self.table_id
        )
    }

    fn search_url(&self) -> String {
        format!(
            "{}/bitable/v1/apps/{}/tables/{}/records/search",
            self.base_url, self.app_token, self.table_id
        )
    }

    fn check(&self, body: &Value, ctx: &str) -> Result<(), SynapticError> {
        let code = body["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            Err(SynapticError::Memory(format!(
                "Lark Bitable MemoryStore ({ctx}) code={code}: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl MemoryStore for LarkBitableMemoryStore {
    async fn append(&self, session_id: &str, message: Message) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;

        let role = message.role().to_string();
        let content = message.content().to_string();
        let tc_slice = message.tool_calls();
        let tool_calls = if tc_slice.is_empty() {
            String::new()
        } else {
            serde_json::to_string(tc_slice).unwrap_or_default()
        };
        let tool_call_id = message.tool_call_id().unwrap_or("").to_string();
        // seq = current unix ms as string for ordering
        let seq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();

        let body = json!({
            "records": [{
                "fields": {
                    "session_id": session_id,
                    "role": role,
                    "content": content,
                    "tool_calls": tool_calls,
                    "tool_call_id": tool_call_id,
                    "seq": seq,
                }
            }]
        });
        let resp = self
            .client
            .post(format!("{}/batch_create", self.records_url()))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable append: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable append parse: {e}")))?;
        self.check(&rb, "append")
    }

    async fn load(&self, session_id: &str) -> Result<Vec<Message>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let body = json!({
            "page_size": 500,
            "filter": {
                "conjunction": "and",
                "conditions": [{
                    "field_name": "session_id",
                    "operator": "is",
                    "value": [session_id]
                }]
            },
            "sort": [{ "field_name": "seq", "desc": false }]
        });
        let resp = self
            .client
            .post(&self.search_url())
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable load: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable load parse: {e}")))?;
        self.check(&rb, "load")?;

        let mut messages = Vec::new();
        for item in rb["data"]["items"].as_array().unwrap_or(&vec![]) {
            let f = &item["fields"];
            let role = f["role"].as_str().unwrap_or("human");
            let content = f["content"].as_str().unwrap_or("").to_string();
            let msg = match role {
                "system" => Message::system(content),
                "ai" | "assistant" => {
                    let tc_str = f["tool_calls"].as_str().unwrap_or("");
                    if tc_str.is_empty() {
                        Message::ai(content)
                    } else {
                        match serde_json::from_str(tc_str) {
                            Ok(tcs) => Message::ai_with_tool_calls(content, tcs),
                            Err(_) => Message::ai(content),
                        }
                    }
                }
                "tool" => {
                    let id = f["tool_call_id"].as_str().unwrap_or("").to_string();
                    Message::tool(id, content)
                }
                _ => Message::human(content),
            };
            messages.push(msg);
        }
        Ok(messages)
    }

    async fn clear(&self, session_id: &str) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let search_body = json!({
            "page_size": 500,
            "filter": {
                "conjunction": "and",
                "conditions": [{
                    "field_name": "session_id",
                    "operator": "is",
                    "value": [session_id]
                }]
            }
        });
        let resp = self
            .client
            .post(&self.search_url())
            .bearer_auth(&token)
            .json(&search_body)
            .send()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable clear search: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable clear parse: {e}")))?;
        self.check(&rb, "clear/search")?;

        let ids: Vec<String> = rb["data"]["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|r| r["record_id"].as_str().map(String::from))
            .collect();

        if ids.is_empty() {
            return Ok(());
        }

        let del_body = json!({ "records": ids });
        let resp = self
            .client
            .delete(format!("{}/batch_delete", self.records_url()))
            .bearer_auth(&token)
            .json(&del_body)
            .send()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable clear delete: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Memory(format!("bitable clear delete parse: {e}")))?;
        self.check(&rb, "clear/delete")
    }
}
