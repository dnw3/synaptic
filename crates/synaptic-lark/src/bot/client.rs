use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use synaptic_core::SynapticError;

use super::streaming::{StreamingCardOptions, StreamingCardWriter};
use crate::{
    api::cardkit::CardKitApi, api::message::MessageApi, api::reaction::ReactionApi,
    auth::TokenCache, LarkConfig,
};

/// Feishu bot client: get bot info, send and reply to messages.
///
/// Supports both one-shot text messages and streaming card output.
pub struct LarkBotClient {
    pub(crate) app_id: String,
    pub(crate) config: LarkConfig,
    token_cache: TokenCache,
    base_url: String,
    client: Client,
    msg_api: MessageApi,
    cardkit_api: CardKitApi,
    reaction_api: ReactionApi,
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
        // MessageApi and CardKitApi get their own token caches (same credentials).
        let msg_api = MessageApi::new(config.clone());
        let cardkit_api = CardKitApi::new(config.clone());
        let reaction_api = ReactionApi::new(config.clone());
        Self {
            app_id,
            config: config.clone(),
            token_cache: config.token_cache(),
            base_url,
            client: Client::new(),
            msg_api,
            cardkit_api,
            reaction_api,
        }
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// GET /bot/v3/info
    pub async fn get_bot_info(&self) -> Result<BotInfo, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/open-apis/bot/v3/info", self.base_url);
        let resp: serde_json::Value = self
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

    /// Send a text message to a chat.  Returns `message_id`.
    pub async fn send_text(
        &self,
        receive_id_type: &str,
        receive_id: &str,
        text: &str,
    ) -> Result<String, SynapticError> {
        let content_json = json!({ "text": text }).to_string();
        self.msg_api
            .send(receive_id_type, receive_id, "text", &content_json)
            .await
    }

    /// Reply to a specific message in its thread.  Returns `message_id`.
    pub async fn reply_text(&self, message_id: &str, text: &str) -> Result<String, SynapticError> {
        let content_json = json!({ "text": text }).to_string();
        self.msg_api.reply(message_id, "text", &content_json).await
    }

    // ── Streaming card methods ──────────────────────────────────────────

    /// Create a card entity.  Returns `card_id`.
    ///
    /// Use this for low-level card operations.  For managed streaming,
    /// prefer [`streaming_send`] or [`streaming_reply`].
    pub async fn create_card(
        &self,
        card_json: &serde_json::Value,
    ) -> Result<String, SynapticError> {
        self.cardkit_api.create(card_json).await
    }

    /// Update a card entity with new content (full card replacement).
    ///
    /// `sequence` must be strictly incrementing per `card_id`.
    /// For streaming text updates, prefer [`stream_card_content`].
    pub async fn update_card(
        &self,
        card_id: &str,
        sequence: i64,
        card_json: &serde_json::Value,
    ) -> Result<(), SynapticError> {
        self.cardkit_api.update(card_id, sequence, card_json).await
    }

    /// Stream text content to a specific element in a card (typewriter effect).
    ///
    /// `element_id` targets a `markdown` or `plain_text` element.
    /// `content` is the **full accumulated text** — if it extends the previous
    /// text, the Feishu client animates the new characters with a typewriter effect.
    ///
    /// `sequence` must be strictly incrementing per `card_id`.
    pub async fn stream_card_content(
        &self,
        card_id: &str,
        element_id: &str,
        content: &str,
        sequence: i64,
    ) -> Result<(), SynapticError> {
        self.cardkit_api
            .stream_content(card_id, element_id, content, sequence)
            .await
    }

    /// Send an interactive card to a chat.  Returns `message_id`.
    pub async fn send_card(
        &self,
        receive_id_type: &str,
        receive_id: &str,
        card_json: &serde_json::Value,
    ) -> Result<String, SynapticError> {
        let content_json = card_json.to_string();
        self.msg_api
            .send(receive_id_type, receive_id, "interactive", &content_json)
            .await
    }

    /// Reply with an interactive card.  Returns `message_id`.
    pub async fn reply_card(
        &self,
        message_id: &str,
        card_json: &serde_json::Value,
    ) -> Result<String, SynapticError> {
        let content_json = card_json.to_string();
        self.msg_api
            .reply(message_id, "interactive", &content_json)
            .await
    }

    /// Start a streaming card and send it to a chat.
    ///
    /// Returns a [`StreamingCardWriter`] that can be used to incrementally
    /// append content.  Call [`StreamingCardWriter::finish`] when done.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let writer = client.streaming_send("chat_id", "oc_xxx", opts).await?;
    /// writer.write("Hello ").await?;
    /// writer.write("World!").await?;
    /// writer.finish().await?;
    /// ```
    pub async fn streaming_send(
        &self,
        receive_id_type: &str,
        receive_id: &str,
        options: StreamingCardOptions,
    ) -> Result<StreamingCardWriter, SynapticError> {
        StreamingCardWriter::send(self.config.clone(), receive_id_type, receive_id, options).await
    }

    /// Start a streaming card as a reply to an existing message.
    ///
    /// Returns a [`StreamingCardWriter`] that can be used to incrementally
    /// append content.  Call [`StreamingCardWriter::finish`] when done.
    pub async fn streaming_reply(
        &self,
        reply_to_message_id: &str,
        options: StreamingCardOptions,
    ) -> Result<StreamingCardWriter, SynapticError> {
        StreamingCardWriter::reply(self.config.clone(), reply_to_message_id, options).await
    }

    // ── Thread / reaction / media ──────────────────────────────────────────

    /// Reply to a message in its thread context (root_id).
    /// Returns `message_id`. Falls back to plain reply if threading is not available.
    pub async fn reply_text_in_thread(
        &self,
        message_id: &str,
        text: &str,
    ) -> Result<String, SynapticError> {
        // Lark's reply API automatically threads if the message_id is a root.
        // We use the same endpoint as reply_text but this method communicates
        // the intent that threading is desired.
        let content_json = serde_json::json!({ "text": text }).to_string();
        self.msg_api.reply(message_id, "text", &content_json).await
    }

    /// Download an image by its `image_key`. Returns the raw bytes.
    pub async fn download_image(&self, image_key: &str) -> Result<Vec<u8>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/open-apis/im/v1/image/{}", self.base_url, image_key);
        let bytes = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("download image: {e}")))?
            .bytes()
            .await
            .map_err(|e| SynapticError::Tool(format!("download image read: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// Download a file resource attached to a message.
    ///
    /// `resource_type` is typically `"file"`, `"image"`, `"audio"`, `"video"`, or `"sticker"`.
    /// Returns the raw bytes.
    pub async fn download_resource(
        &self,
        message_id: &str,
        file_key: &str,
        resource_type: &str,
    ) -> Result<Vec<u8>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/open-apis/im/v1/messages/{message_id}/resources/{file_key}?type={resource_type}",
            self.base_url
        );
        let bytes = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("download resource: {e}")))?
            .bytes()
            .await
            .map_err(|e| SynapticError::Tool(format!("download resource read: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// Add an emoji reaction to a message. Returns the `reaction_id`.
    pub async fn add_reaction(
        &self,
        message_id: &str,
        emoji_type: &str,
    ) -> Result<String, SynapticError> {
        self.reaction_api.add_reaction(message_id, emoji_type).await
    }

    /// Remove an emoji reaction from a message.
    pub async fn remove_reaction(
        &self,
        message_id: &str,
        reaction_id: &str,
    ) -> Result<(), SynapticError> {
        self.reaction_api
            .delete_reaction(message_id, reaction_id)
            .await
    }
}
