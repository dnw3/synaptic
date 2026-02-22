use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Lightweight internal HTTP helper for the Feishu IM Chat API.
pub(crate) struct ChatApi {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl ChatApi {
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub(crate) fn check(body: &Value, ctx: &str) -> Result<(), SynapticError> {
        let code = body["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            Err(SynapticError::Tool(format!(
                "Lark Chat API error ({ctx}) code={code}: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }

    /// List all chats the bot belongs to.
    pub async fn list_chats(
        &self,
        page_token: Option<&str>,
    ) -> Result<(Vec<Value>, Option<String>), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let mut url = format!("{}/im/v1/chats?page_size=50", self.base_url);
        if let Some(pt) = page_token {
            url.push_str(&format!("&page_token={pt}"));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat list_chats: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat list_chats parse: {e}")))?;
        Self::check(&body, "list_chats")?;
        let items = body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let next = body["data"]["page_token"].as_str().map(String::from);
        let has_more = body["data"]["has_more"].as_bool().unwrap_or(false);
        Ok((items, if has_more { next } else { None }))
    }

    /// Get a single chat by ID.
    pub async fn get_chat(&self, chat_id: &str) -> Result<Value, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/chats/{chat_id}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat get_chat: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat get_chat parse: {e}")))?;
        Self::check(&body, "get_chat")?;
        Ok(body["data"].clone())
    }

    /// Create a new group chat and return the chat_id.
    pub async fn create_chat(
        &self,
        name: &str,
        description: Option<&str>,
        open_ids: &[String],
    ) -> Result<String, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/chats", self.base_url);
        let members: Vec<Value> = open_ids
            .iter()
            .map(|id| json!({ "member_id_type": "open_id", "member_id": id }))
            .collect();
        let mut body = json!({ "name": name, "members": members });
        if let Some(desc) = description {
            body["description"] = json!(desc);
        }
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat create_chat: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat create_chat parse: {e}")))?;
        Self::check(&rb, "create_chat")?;
        Ok(rb["data"]["chat_id"].as_str().unwrap_or("").to_string())
    }

    /// Update chat name and/or description.
    pub async fn update_chat(
        &self,
        chat_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/chats/{chat_id}", self.base_url);
        let mut body = json!({});
        if let Some(n) = name {
            body["name"] = json!(n);
        }
        if let Some(d) = description {
            body["description"] = json!(d);
        }
        let resp = self
            .client
            .put(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat update_chat: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat update_chat parse: {e}")))?;
        Self::check(&rb, "update_chat")
    }

    /// List members of a chat.
    pub async fn list_members(&self, chat_id: &str) -> Result<Vec<Value>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/chats/{chat_id}/members", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat list_members: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat list_members parse: {e}")))?;
        Self::check(&body, "list_members")?;
        Ok(body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Add members to a chat.
    pub async fn add_members(
        &self,
        chat_id: &str,
        open_ids: &[String],
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/chats/{chat_id}/members", self.base_url);
        let members: Vec<Value> = open_ids
            .iter()
            .map(|id| json!({ "member_id_type": "open_id", "member_id": id }))
            .collect();
        let body = json!({ "members": members });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat add_members: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat add_members parse: {e}")))?;
        Self::check(&rb, "add_members")
    }

    /// Remove members from a chat.
    pub async fn remove_members(
        &self,
        chat_id: &str,
        open_ids: &[String],
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/im/v1/chats/{chat_id}/members", self.base_url);
        let members: Vec<Value> = open_ids
            .iter()
            .map(|id| json!({ "member_id_type": "open_id", "member_id": id }))
            .collect();
        let body = json!({ "members": members });
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat remove_members: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("chat remove_members parse: {e}")))?;
        Self::check(&rb, "remove_members")
    }
}
