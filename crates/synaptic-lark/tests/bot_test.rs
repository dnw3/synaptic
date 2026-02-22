#![cfg(feature = "bot")]

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use synaptic_core::SynapticError;
use synaptic_lark::{
    bot::{LarkBotClient, LarkLongConnListener, LarkMessageEvent, MessageHandler},
    LarkConfig,
};

#[test]
fn bot_client_stores_config() {
    let client = LarkBotClient::new(LarkConfig::new("cli_test", "secret_test"));
    assert_eq!(client.app_id(), "cli_test");
}

#[test]
fn long_conn_listener_builder() {
    let listener = LarkLongConnListener::new(LarkConfig::new("a", "b")).with_dedup_capacity(256);
    assert_eq!(listener.dedup_capacity(), 256);
}

#[test]
fn message_event_text_extraction() {
    let event_json = serde_json::json!({
        "schema": "2.0",
        "header": {
            "event_id": "evt001",
            "event_type": "im.message.receive_v1",
            "app_id": "cli_xxx",
            "tenant_key": "xxx"
        },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_xxx" } },
            "message": {
                "message_id": "om_xxx",
                "chat_id": "oc_xxx",
                "message_type": "text",
                "content": r#"{"text":"hello bot"}"#
            }
        }
    });
    let msg_event = LarkMessageEvent::from_payload(&event_json).unwrap();
    assert_eq!(msg_event.text(), "hello bot");
    assert_eq!(msg_event.chat_id(), "oc_xxx");
    assert_eq!(msg_event.message_id(), "om_xxx");
    assert_eq!(msg_event.sender_open_id(), "ou_xxx");
}

struct EchoHandler {
    received: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl MessageHandler for EchoHandler {
    async fn handle(
        &self,
        event: LarkMessageEvent,
        _client: &LarkBotClient,
    ) -> Result<(), SynapticError> {
        self.received.lock().unwrap().push(event.text().to_string());
        Ok(())
    }
}

#[tokio::test]
async fn dispatch_to_message_handler() {
    let received = Arc::new(Mutex::new(vec![]));
    let handler = EchoHandler {
        received: received.clone(),
    };

    let listener =
        LarkLongConnListener::new(LarkConfig::new("a", "b")).with_message_handler(handler);

    let payload = serde_json::json!({
        "schema": "2.0",
        "header": {
            "event_id": "evt-unique-001",
            "event_type": "im.message.receive_v1"
        },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_abc" } },
            "message": {
                "message_id": "om_abc",
                "chat_id": "oc_abc",
                "message_type": "text",
                "content": r#"{"text":"test message"}"#
            }
        }
    });
    listener.dispatch_payload(&payload).await.unwrap();
    // Give the spawned tokio task a moment to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    assert_eq!(received.lock().unwrap().as_slice(), &["test message"]);
}

#[tokio::test]
async fn dedup_same_event_id() {
    let received = Arc::new(Mutex::new(0u32));
    let counter = received.clone();

    struct CountHandler(Arc<Mutex<u32>>);

    #[async_trait]
    impl MessageHandler for CountHandler {
        async fn handle(
            &self,
            _e: LarkMessageEvent,
            _c: &LarkBotClient,
        ) -> Result<(), SynapticError> {
            *self.0.lock().unwrap() += 1;
            Ok(())
        }
    }

    let listener = LarkLongConnListener::new(LarkConfig::new("a", "b"))
        .with_message_handler(CountHandler(counter));

    let payload = serde_json::json!({
        "schema": "2.0",
        "header": { "event_id": "evt-dup-001", "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_x" } },
            "message": {
                "message_id": "om_x", "chat_id": "oc_x",
                "message_type": "text", "content": r#"{"text":"hi"}"#
            }
        }
    });

    // Dispatch same event_id twice (Lark retry scenario)
    listener.dispatch_payload(&payload).await.unwrap();
    listener.dispatch_payload(&payload).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert_eq!(
        *received.lock().unwrap(),
        1,
        "handler must be called exactly once per event_id"
    );
}
