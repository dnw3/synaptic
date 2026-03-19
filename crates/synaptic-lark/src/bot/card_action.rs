use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::SynapticError;

use super::client::LarkBotClient;

/// Parsed representation of a `card.action.trigger` event.
#[derive(Debug, Clone)]
pub struct CardActionEvent {
    /// The open_id of the user who performed the action.
    pub operator_open_id: String,
    /// The message ID of the card that was interacted with.
    pub message_id: String,
    /// The chat ID where the card lives.
    pub chat_id: String,
    /// The action tag, e.g. `"button"`, `"select_static"`, `"overflow"`, etc.
    pub action_tag: String,
    /// The value attached to the action element.
    pub action_value: Value,
    /// Full raw payload for advanced handlers.
    pub raw: Value,
}

impl CardActionEvent {
    /// Parse from a full v2.0 `card.action.trigger` event payload.
    pub fn from_payload(payload: &Value) -> Result<Self, SynapticError> {
        let event = &payload["event"];

        let operator_open_id = event["operator"]["open_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let context = &event["context"];
        let message_id = context["open_message_id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let chat_id = context["open_chat_id"].as_str().unwrap_or("").to_string();

        let action = &event["action"];
        let action_tag = action["tag"].as_str().unwrap_or("").to_string();
        let action_value = action["value"].clone();

        Ok(Self {
            operator_open_id,
            message_id,
            chat_id,
            action_tag,
            action_value,
            raw: payload.clone(),
        })
    }
}

/// Handler trait for interactive card action events.
#[async_trait]
pub trait CardActionHandler: Send + Sync {
    async fn handle(
        &self,
        event: CardActionEvent,
        client: &LarkBotClient,
    ) -> Result<(), SynapticError>;
}
