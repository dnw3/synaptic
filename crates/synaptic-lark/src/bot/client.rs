use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Feishu bot client: get bot info, send and reply to messages.
pub struct LarkBotClient {
    pub(crate) app_id: String,
    token_cache: TokenCache,
    base_url: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct BotInfo {
    pub app_name: String,
    pub avatar_url: String,
    pub ip_white_list: Vec<String>,
    pub open_id: String,
}

impl LarkBotClient {
    pub fn new(config: LarkConfig) -> Self {
        let app_id = config.app_id.clone();
        let base_url = config.base_url.clone();
        Self {
            app_id,
            token_cache: config.token_cache(),
            base_url,
            client: Client::new(),
        }
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// GET /bot/v3/info
    pub async fn get_bot_info(&self) -> Result<BotInfo, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/bot/v3/info", self.base_url);
        let resp: Value = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("get_bot_info: {e}")))?
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("get_bot_info parse: {e}")))?;
        if resp["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "get_bot_info error: {}",
                resp["msg"].as_str().unwrap_or("unknown")
            )));
        }
        let bot = &resp["bot"];
        Ok(BotInfo {
            app_name: bot["app_name"].as_str().unwrap_or("").to_string(),
            avatar_url: bot["avatar_url"].as_str().unwrap_or("").to_string(),
            ip_white_list: bot["ip_white_list"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            open_id: bot["open_id"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Send a text message to a chat.
    pub async fn send_text(
        &self,
        receive_id_type: &str,
        receive_id: &str,
        text: &str,
    ) -> Result<String, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/im/v1/messages?receive_id_type={receive_id_type}",
            self.base_url
        );
        let resp: Value = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&json!({
                "receive_id": receive_id,
                "msg_type": "text",
                "content": json!({ "text": text }).to_string()
            }))
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("send_text: {e}")))?
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("send_text parse: {e}")))?;
        if resp["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "send_text error: {}",
                resp["msg"].as_str().unwrap_or("unknown")
            )));
        }
        Ok(resp["data"]["message_id"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Reply to a specific message (thread reply).
    pub async fn reply_text(&self, message_id: &str, text: &str) -> Result<String, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/messages/{message_id}/reply", self.base_url);
        let resp: Value = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&json!({
                "msg_type": "text",
                "content": json!({ "text": text }).to_string()
            }))
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("reply_text: {e}")))?
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("reply_text parse: {e}")))?;
        if resp["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "reply_text error: {}",
                resp["msg"].as_str().unwrap_or("unknown")
            )));
        }
        Ok(resp["data"]["message_id"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
