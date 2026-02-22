use serde_json::Value;
use synaptic_core::SynapticError;

/// Parsed representation of a `im.message.receive_v1` event.
#[derive(Debug, Clone)]
pub struct LarkMessageEvent {
    pub event_id: String,
    pub message_id: String,
    pub chat_id: String,
    pub sender_open_id: String,
    pub message_type: String,
    /// Extracted plain text (for text messages) or raw content JSON string.
    pub text: String,
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
        let message_type = msg["message_type"].as_str().unwrap_or("text").to_string();
        let sender_open_id = payload["event"]["sender"]["sender_id"]["open_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Extract plain text from content JSON
        let content_str = msg["content"].as_str().unwrap_or("{}");
        let content: Value = serde_json::from_str(content_str).unwrap_or(Value::Null);
        let text = content["text"].as_str().unwrap_or(content_str).to_string();

        Ok(Self {
            event_id,
            message_id,
            chat_id,
            sender_open_id,
            message_type,
            text,
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
}
