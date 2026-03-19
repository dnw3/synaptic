use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::SynapticError;

use super::card_action::CardActionEvent;
use super::client::LarkBotClient;
use super::session::LarkMessageEvent;

// ---------------------------------------------------------------------------
// Unified event enum
// ---------------------------------------------------------------------------

/// Unified enum covering all Lark v2 event types that the bot can receive.
#[derive(Debug, Clone)]
pub enum LarkEvent {
    /// A new message was received (`im.message.receive_v1`).
    Message(LarkMessageEvent),
    /// A message was deleted (`im.message.deleted_v1`).
    MessageDeleted(MessageDeletedEvent),
    /// A reaction was added to a message (`im.message.reaction.created_v1`).
    ReactionCreated(ReactionEvent),
    /// A reaction was removed from a message (`im.message.reaction.deleted_v1`).
    ReactionDeleted(ReactionEvent),
    /// A message was read (`im.message.read_v1`).
    MessageRead(MessageReadEvent),
    /// The bot was added to a group (`im.chat.member.bot.added_v1`).
    BotAdded(BotLifecycleEvent),
    /// The bot was removed from a group (`im.chat.member.bot.deleted_v1`).
    BotRemoved(BotLifecycleEvent),
    /// A group was disbanded (`im.chat.disbanded_v1`).
    GroupDisbanded(GroupDisbandedEvent),
    /// Group settings were updated (`im.chat.updated_v1`).
    GroupUpdated(GroupUpdatedEvent),
    /// An interactive card action was triggered (`card.action.trigger`).
    CardAction(CardActionEvent),
    /// A bot menu item was clicked (`application.bot.menu_v6`).
    BotMenuClick(BotMenuEvent),
    /// An event type not explicitly handled.
    Unknown { event_type: String, raw: Value },
}

// ---------------------------------------------------------------------------
// Event structs
// ---------------------------------------------------------------------------

/// A message was deleted.
#[derive(Debug, Clone)]
pub struct MessageDeletedEvent {
    pub event_id: String,
    pub message_id: String,
    pub chat_id: String,
    pub raw: Value,
}

/// A reaction was added or removed on a message.
#[derive(Debug, Clone)]
pub struct ReactionEvent {
    pub event_id: String,
    pub message_id: String,
    pub chat_id: String,
    pub operator_open_id: String,
    pub emoji_type: String,
    pub raw: Value,
}

/// One or more messages were read.
#[derive(Debug, Clone)]
pub struct MessageReadEvent {
    pub event_id: String,
    pub reader_open_id: String,
    pub message_ids: Vec<String>,
    pub chat_id: String,
    pub raw: Value,
}

/// The bot was added to or removed from a group.
#[derive(Debug, Clone)]
pub struct BotLifecycleEvent {
    pub event_id: String,
    pub chat_id: String,
    pub operator_open_id: String,
    pub raw: Value,
}

/// A group was disbanded.
#[derive(Debug, Clone)]
pub struct GroupDisbandedEvent {
    pub event_id: String,
    pub chat_id: String,
    pub operator_open_id: String,
    pub raw: Value,
}

/// Group metadata was updated (e.g. name change).
#[derive(Debug, Clone)]
pub struct GroupUpdatedEvent {
    pub event_id: String,
    pub chat_id: String,
    pub chat_name: Option<String>,
    pub operator_open_id: String,
    pub raw: Value,
}

/// A bot-menu button was clicked.
#[derive(Debug, Clone)]
pub struct BotMenuEvent {
    pub event_id: String,
    pub operator_open_id: String,
    pub event_key: String,
    pub raw: Value,
}

// ---------------------------------------------------------------------------
// Handler traits
// ---------------------------------------------------------------------------

/// Generic handler for any [`LarkEvent`].
#[async_trait]
pub trait LarkEventHandler: Send + Sync {
    async fn handle(&self, event: LarkEvent, client: &LarkBotClient) -> Result<(), SynapticError>;
}

/// Handler for a specific event type identified by its Lark `event_type` string.
/// Useful for extending the event system with custom / uncommon event types.
#[async_trait]
pub trait CustomEventHandler: Send + Sync {
    /// The Lark event_type string this handler is interested in (e.g. `"contact.user.created_v3"`).
    fn event_type(&self) -> &str;

    async fn handle(&self, payload: &Value, client: &LarkBotClient) -> Result<(), SynapticError>;
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Extract event_id from the standard header location.
fn extract_event_id(payload: &Value) -> String {
    payload["header"]["event_id"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

/// Extract operator open_id — Lark uses both `operator_id.open_id` and `operator.open_id`.
fn extract_operator(event: &Value) -> String {
    event["operator_id"]["open_id"]
        .as_str()
        .or_else(|| event["operator"]["open_id"].as_str())
        .unwrap_or("")
        .to_string()
}

// ---------------------------------------------------------------------------
// parse_event
// ---------------------------------------------------------------------------

/// Parse a raw Lark v2 event payload into a typed [`LarkEvent`].
pub fn parse_event(payload: &Value) -> Result<LarkEvent, SynapticError> {
    let event_type = payload["header"]["event_type"].as_str().unwrap_or("");

    match event_type {
        // ── Messages ─────────────────────────────────────────────────
        "im.message.receive_v1" => Ok(LarkEvent::Message(LarkMessageEvent::from_payload(payload)?)),

        "im.message.deleted_v1" => {
            let ev = &payload["event"];
            Ok(LarkEvent::MessageDeleted(MessageDeletedEvent {
                event_id: extract_event_id(payload),
                message_id: ev["message_id"].as_str().unwrap_or("").to_string(),
                chat_id: ev["chat_id"].as_str().unwrap_or("").to_string(),
                raw: payload.clone(),
            }))
        }

        // ── Reactions ────────────────────────────────────────────────
        "im.message.reaction.created_v1" => Ok(LarkEvent::ReactionCreated(parse_reaction(payload))),
        "im.message.reaction.deleted_v1" => Ok(LarkEvent::ReactionDeleted(parse_reaction(payload))),

        // ── Read receipts ────────────────────────────────────────────
        "im.message.read_v1" => {
            let ev = &payload["event"];
            let reader_open_id = ev["reader"]["reader_id"]["open_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let message_ids = ev["message_id_list"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let chat_id = ev["chat_id"].as_str().unwrap_or("").to_string();
            Ok(LarkEvent::MessageRead(MessageReadEvent {
                event_id: extract_event_id(payload),
                reader_open_id,
                message_ids,
                chat_id,
                raw: payload.clone(),
            }))
        }

        // ── Bot lifecycle ────────────────────────────────────────────
        "im.chat.member.bot.added_v1" => Ok(LarkEvent::BotAdded(parse_bot_lifecycle(payload))),
        "im.chat.member.bot.deleted_v1" => Ok(LarkEvent::BotRemoved(parse_bot_lifecycle(payload))),

        // ── Group events ─────────────────────────────────────────────
        "im.chat.disbanded_v1" => {
            let ev = &payload["event"];
            Ok(LarkEvent::GroupDisbanded(GroupDisbandedEvent {
                event_id: extract_event_id(payload),
                chat_id: ev["chat_id"].as_str().unwrap_or("").to_string(),
                operator_open_id: extract_operator(ev),
                raw: payload.clone(),
            }))
        }

        "im.chat.updated_v1" => {
            let ev = &payload["event"];
            let chat_name = ev["after_change"]["chat_name"].as_str().map(String::from);
            Ok(LarkEvent::GroupUpdated(GroupUpdatedEvent {
                event_id: extract_event_id(payload),
                chat_id: ev["chat_id"].as_str().unwrap_or("").to_string(),
                chat_name,
                operator_open_id: extract_operator(ev),
                raw: payload.clone(),
            }))
        }

        // ── Card actions ─────────────────────────────────────────────
        "card.action.trigger" => Ok(LarkEvent::CardAction(CardActionEvent::from_payload(
            payload,
        )?)),

        // ── Bot menu ─────────────────────────────────────────────────
        "application.bot.menu_v6" => {
            let ev = &payload["event"];
            Ok(LarkEvent::BotMenuClick(BotMenuEvent {
                event_id: extract_event_id(payload),
                operator_open_id: extract_operator(ev),
                event_key: ev["event_key"].as_str().unwrap_or("").to_string(),
                raw: payload.clone(),
            }))
        }

        // ── Fallback ────────────────────────────────────────────────
        other => Ok(LarkEvent::Unknown {
            event_type: other.to_string(),
            raw: payload.clone(),
        }),
    }
}

fn parse_reaction(payload: &Value) -> ReactionEvent {
    let ev = &payload["event"];
    ReactionEvent {
        event_id: extract_event_id(payload),
        message_id: ev["message_id"].as_str().unwrap_or("").to_string(),
        chat_id: ev["chat_id"].as_str().unwrap_or("").to_string(),
        operator_open_id: extract_operator(ev),
        emoji_type: ev["reaction_type"]["emoji_type"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        raw: payload.clone(),
    }
}

fn parse_bot_lifecycle(payload: &Value) -> BotLifecycleEvent {
    let ev = &payload["event"];
    BotLifecycleEvent {
        event_id: extract_event_id(payload),
        chat_id: ev["chat_id"].as_str().unwrap_or("").to_string(),
        operator_open_id: extract_operator(ev),
        raw: payload.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn header(event_type: &str) -> Value {
        json!({
            "event_id": "ev_test_001",
            "event_type": event_type
        })
    }

    #[test]
    fn parse_message_event() {
        let payload = json!({
            "header": header("im.message.receive_v1"),
            "event": {
                "message": {
                    "message_id": "msg_001",
                    "chat_id": "oc_abc",
                    "chat_type": "p2p",
                    "message_type": "text",
                    "content": r#"{"text":"hello"}"#,
                    "mentions": []
                },
                "sender": { "sender_id": { "open_id": "ou_sender" } }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::Message(msg) => {
                assert_eq!(msg.message_id, "msg_001");
                assert_eq!(msg.text(), "hello");
            }
            other => panic!("expected Message, got {:?}", other),
        }
    }

    #[test]
    fn parse_message_deleted() {
        let payload = json!({
            "header": header("im.message.deleted_v1"),
            "event": {
                "message_id": "msg_del",
                "chat_id": "oc_del"
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::MessageDeleted(e) => {
                assert_eq!(e.event_id, "ev_test_001");
                assert_eq!(e.message_id, "msg_del");
                assert_eq!(e.chat_id, "oc_del");
            }
            other => panic!("expected MessageDeleted, got {:?}", other),
        }
    }

    #[test]
    fn parse_reaction_created() {
        let payload = json!({
            "header": header("im.message.reaction.created_v1"),
            "event": {
                "message_id": "msg_r",
                "chat_id": "oc_r",
                "operator_id": { "open_id": "ou_reactor" },
                "reaction_type": { "emoji_type": "THUMBSUP" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::ReactionCreated(e) => {
                assert_eq!(e.message_id, "msg_r");
                assert_eq!(e.operator_open_id, "ou_reactor");
                assert_eq!(e.emoji_type, "THUMBSUP");
            }
            other => panic!("expected ReactionCreated, got {:?}", other),
        }
    }

    #[test]
    fn parse_reaction_deleted() {
        let payload = json!({
            "header": header("im.message.reaction.deleted_v1"),
            "event": {
                "message_id": "msg_r2",
                "chat_id": "oc_r2",
                "operator_id": { "open_id": "ou_r2" },
                "reaction_type": { "emoji_type": "SMILE" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::ReactionDeleted(e) => {
                assert_eq!(e.emoji_type, "SMILE");
                assert_eq!(e.chat_id, "oc_r2");
            }
            other => panic!("expected ReactionDeleted, got {:?}", other),
        }
    }

    #[test]
    fn parse_message_read() {
        let payload = json!({
            "header": header("im.message.read_v1"),
            "event": {
                "reader": { "reader_id": { "open_id": "ou_reader" } },
                "message_id_list": ["msg_a", "msg_b"],
                "chat_id": "oc_read"
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::MessageRead(e) => {
                assert_eq!(e.reader_open_id, "ou_reader");
                assert_eq!(e.message_ids, vec!["msg_a", "msg_b"]);
                assert_eq!(e.chat_id, "oc_read");
            }
            other => panic!("expected MessageRead, got {:?}", other),
        }
    }

    #[test]
    fn parse_bot_added() {
        let payload = json!({
            "header": header("im.chat.member.bot.added_v1"),
            "event": {
                "chat_id": "oc_grp",
                "operator_id": { "open_id": "ou_adder" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::BotAdded(e) => {
                assert_eq!(e.chat_id, "oc_grp");
                assert_eq!(e.operator_open_id, "ou_adder");
            }
            other => panic!("expected BotAdded, got {:?}", other),
        }
    }

    #[test]
    fn parse_bot_removed() {
        let payload = json!({
            "header": header("im.chat.member.bot.deleted_v1"),
            "event": {
                "chat_id": "oc_grp2",
                "operator_id": { "open_id": "ou_remover" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::BotRemoved(e) => {
                assert_eq!(e.chat_id, "oc_grp2");
                assert_eq!(e.operator_open_id, "ou_remover");
            }
            other => panic!("expected BotRemoved, got {:?}", other),
        }
    }

    #[test]
    fn parse_group_disbanded() {
        let payload = json!({
            "header": header("im.chat.disbanded_v1"),
            "event": {
                "chat_id": "oc_disband",
                "operator_id": { "open_id": "ou_disbander" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::GroupDisbanded(e) => {
                assert_eq!(e.chat_id, "oc_disband");
                assert_eq!(e.operator_open_id, "ou_disbander");
            }
            other => panic!("expected GroupDisbanded, got {:?}", other),
        }
    }

    #[test]
    fn parse_group_updated() {
        let payload = json!({
            "header": header("im.chat.updated_v1"),
            "event": {
                "chat_id": "oc_upd",
                "operator_id": { "open_id": "ou_updater" },
                "after_change": { "chat_name": "New Name" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::GroupUpdated(e) => {
                assert_eq!(e.chat_id, "oc_upd");
                assert_eq!(e.chat_name.as_deref(), Some("New Name"));
                assert_eq!(e.operator_open_id, "ou_updater");
            }
            other => panic!("expected GroupUpdated, got {:?}", other),
        }
    }

    #[test]
    fn parse_group_updated_no_name_change() {
        let payload = json!({
            "header": header("im.chat.updated_v1"),
            "event": {
                "chat_id": "oc_upd2",
                "operator_id": { "open_id": "ou_u2" },
                "after_change": {}
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::GroupUpdated(e) => {
                assert!(e.chat_name.is_none());
            }
            other => panic!("expected GroupUpdated, got {:?}", other),
        }
    }

    #[test]
    fn parse_card_action() {
        let payload = json!({
            "header": header("card.action.trigger"),
            "event": {
                "operator": { "open_id": "ou_clicker" },
                "context": {
                    "open_message_id": "msg_card",
                    "open_chat_id": "oc_card"
                },
                "action": {
                    "tag": "button",
                    "value": { "key": "approve" }
                }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::CardAction(e) => {
                assert_eq!(e.operator_open_id, "ou_clicker");
                assert_eq!(e.action_tag, "button");
            }
            other => panic!("expected CardAction, got {:?}", other),
        }
    }

    #[test]
    fn parse_bot_menu() {
        let payload = json!({
            "header": header("application.bot.menu_v6"),
            "event": {
                "operator": { "open_id": "ou_menu" },
                "event_key": "help_menu"
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::BotMenuClick(e) => {
                assert_eq!(e.operator_open_id, "ou_menu");
                assert_eq!(e.event_key, "help_menu");
            }
            other => panic!("expected BotMenuClick, got {:?}", other),
        }
    }

    #[test]
    fn parse_unknown_event() {
        let payload = json!({
            "header": { "event_id": "ev_unk", "event_type": "contact.user.created_v3" },
            "event": { "user_id": "ou_new" }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::Unknown { event_type, .. } => {
                assert_eq!(event_type, "contact.user.created_v3");
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn parse_missing_event_type() {
        let payload = json!({ "header": {}, "event": {} });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::Unknown { event_type, .. } => {
                assert_eq!(event_type, "");
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn operator_id_fallback() {
        // Tests that extract_operator tries operator_id first, then operator
        let payload = json!({
            "header": header("im.chat.disbanded_v1"),
            "event": {
                "chat_id": "oc_fb",
                "operator": { "open_id": "ou_via_operator" }
            }
        });
        let event = parse_event(&payload).unwrap();
        match event {
            LarkEvent::GroupDisbanded(e) => {
                assert_eq!(e.operator_open_id, "ou_via_operator");
            }
            other => panic!("expected GroupDisbanded, got {:?}", other),
        }
    }
}
