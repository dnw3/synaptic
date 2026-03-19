use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Lightweight internal HTTP helper for the Feishu IM (reaction) API.
///
/// Supports adding and removing emoji reactions on messages.
pub(crate) struct ReactionApi {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl ReactionApi {
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Add an emoji reaction to a message.  Returns `reaction_id`.
    ///
    /// POST /im/v1/messages/{message_id}/reactions
    pub async fn add_reaction(
        &self,
        message_id: &str,
        emoji_type: &str,
    ) -> Result<String, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/open-apis/im/v1/messages/{message_id}/reactions",
            self.base_url
        );
        let body = json!({
            "reaction_type": { "emoji_type": emoji_type }
        });
        let resp: Value = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("add reaction: {e}")))?
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("add reaction parse: {e}")))?;
        check_code(&resp, "add_reaction")?;
        Ok(resp["data"]["reaction_id"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Remove an emoji reaction from a message.
    ///
    /// DELETE /im/v1/messages/{message_id}/reactions/{reaction_id}
    pub async fn delete_reaction(
        &self,
        message_id: &str,
        reaction_id: &str,
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/open-apis/im/v1/messages/{message_id}/reactions/{reaction_id}",
            self.base_url
        );
        let resp: Value = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("delete reaction: {e}")))?
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("delete reaction parse: {e}")))?;
        check_code(&resp, "delete_reaction")
    }
}

fn check_code(body: &Value, ctx: &str) -> Result<(), SynapticError> {
    let code = body["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        Err(SynapticError::Tool(format!(
            "Lark reaction API error ({ctx}) code={code}: {}",
            body["msg"].as_str().unwrap_or("unknown")
        )))
    } else {
        Ok(())
    }
}
