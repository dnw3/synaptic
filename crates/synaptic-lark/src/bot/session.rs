use serde::{Deserialize, Serialize};
use serde_json::Value;
use synaptic_core::SynapticError;

/// A @mention extracted from a Lark message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionInfo {
    /// The placeholder key in the text (e.g. `@_user_1`).
    pub key: String,
    /// The open_id of the mentioned user or bot.
    pub id: String,
    /// The name of the mentioned user or bot.
    pub name: String,
    /// Whether this mention refers to the bot itself.
    pub is_bot: bool,
}

/// Parsed representation of a `im.message.receive_v1` event.
#[derive(Debug, Clone)]
pub struct LarkMessageEvent {
    pub event_id: String,
    pub message_id: String,
    pub chat_id: String,
    pub sender_open_id: String,
    pub message_type: String,
    /// Chat type: "p2p" (DM) or "group".
    pub chat_type: String,
    /// Parent message ID for threaded replies.
    pub root_id: Option<String>,
    /// Extracted plain text (for text messages) or raw content JSON string.
    pub text: String,
    /// @mentions extracted from the message.
    pub mentions: Vec<MentionInfo>,
    /// Full raw payload for advanced handlers.
    pub raw: Value,
}

impl LarkMessageEvent {
    /// Parse from a full v2.0 event payload.
    pub fn from_payload(payload: &Value) -> Result<Self, SynapticError> {
        let event_id = payload["header"]["event_id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let msg = &payload["event"]["message"];
        let message_id = msg["message_id"].as_str().unwrap_or("").to_string();
        let chat_id = msg["chat_id"].as_str().unwrap_or("").to_string();
        let chat_type = msg["chat_type"].as_str().unwrap_or("p2p").to_string();
        let message_type = msg["message_type"].as_str().unwrap_or("text").to_string();
        let root_id = msg["root_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let sender_open_id = payload["event"]["sender"]["sender_id"]["open_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Extract plain text from content JSON
        let content_str = msg["content"].as_str().unwrap_or("{}");
        let content: Value = serde_json::from_str(content_str).unwrap_or(Value::Null);
        let text = content["text"].as_str().unwrap_or(content_str).to_string();

        // Extract mentions
        let mentions = msg["mentions"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|m| {
                let key = m["key"].as_str().unwrap_or("").to_string();
                let id = m["id"]["open_id"].as_str().unwrap_or("").to_string();
                let name = m["name"].as_str().unwrap_or("").to_string();
                MentionInfo {
                    key,
                    id,
                    name,
                    is_bot: false, // updated via is_bot field if present
                }
            })
            .collect();

        Ok(Self {
            event_id,
            message_id,
            chat_id,
            chat_type,
            sender_open_id,
            message_type,
            root_id,
            text,
            mentions,
            raw: payload.clone(),
        })
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    pub fn message_id(&self) -> &str {
        &self.message_id
    }
    pub fn chat_id(&self) -> &str {
        &self.chat_id
    }
    pub fn sender_open_id(&self) -> &str {
        &self.sender_open_id
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns true if this is a DM (p2p) message.
    pub fn is_dm(&self) -> bool {
        self.chat_type == "p2p"
    }

    /// Returns true if this is a group message.
    pub fn is_group(&self) -> bool {
        self.chat_type == "group"
    }

    /// Returns true if this message is part of a thread.
    pub fn has_thread(&self) -> bool {
        self.root_id.is_some()
    }

    /// Returns true if this is a text message.
    pub fn is_text(&self) -> bool {
        self.message_type == "text"
    }

    /// Returns true if this is an image message.
    pub fn is_image(&self) -> bool {
        self.message_type == "image"
    }

    /// Returns true if this is a file message.
    pub fn is_file(&self) -> bool {
        self.message_type == "file"
    }

    /// Returns the message type as a string.
    pub fn message_type_str(&self) -> &str {
        &self.message_type
    }

    /// Returns true if the bot (given its open_id) is mentioned.
    pub fn mentions_bot(&self, bot_open_id: &str) -> bool {
        self.mentions.iter().any(|m| m.id == bot_open_id)
    }

    /// Returns the image key if this is an image message.
    pub fn image_key(&self) -> Option<String> {
        if self.message_type == "image" {
            let content_str = self.raw["event"]["message"]["content"]
                .as_str()
                .unwrap_or("{}");
            let content: Value = serde_json::from_str(content_str).unwrap_or(Value::Null);
            content["image_key"].as_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Returns the file key if this is a file message.
    pub fn file_key(&self) -> Option<String> {
        if self.message_type == "file" {
            let content_str = self.raw["event"]["message"]["content"]
                .as_str()
                .unwrap_or("{}");
            let content: Value = serde_json::from_str(content_str).unwrap_or(Value::Null);
            content["file_key"].as_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Returns the file name if this is a file message.
    /// Note: returns a static &str; callers should use .unwrap_or("file").
    pub fn file_name(&self) -> Option<&'static str> {
        if self.message_type == "file" {
            Some("file")
        } else {
            None
        }
    }
}
