# Messaging & Bot

## LarkMessageTool

Send messages to Feishu chats or users as an Agent tool.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkMessageTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkMessageTool::new(config);

// Text message
let result = tool.call(json!({
    "action": "send",
    "receive_id_type": "chat_id",
    "receive_id": "oc_xxx",
    "msg_type": "text",
    "content": "Hello from Synaptic Agent!"
})).await?;

println!("Sent message ID: {}", result["message_id"]);
```

### Actions

| Action | Required fields | Description |
|--------|----------------|-------------|
| `send` (default) | `receive_id_type`, `receive_id`, `msg_type`, `content` | Send a new message |
| `update` | `message_id`, `msg_type`, `content` | Update an existing message |
| `delete` | `message_id` | Delete (recall) a message |

### Parameters

| Field | Type | Description |
|-------|------|-------------|
| `receive_id_type` | string | `"chat_id"` \| `"user_id"` \| `"email"` \| `"open_id"` |
| `receive_id` | string | The receiver ID matching the type |
| `msg_type` | string | `"text"` \| `"post"` (rich text) \| `"interactive"` (card) |
| `content` | string | Plain string for text; JSON string for post/interactive |

---

## LarkEventListener

Subscribe to Feishu webhook events with HMAC-SHA256 signature verification and automatic URL challenge handling. Register typed event handlers by event name.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkEventListener};

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let listener = LarkEventListener::new(config)
    .on("im.message.receive_v1", |event| async move {
        let msg = &event["event"]["message"]["content"];
        println!("Received: {}", msg);
        Ok(())
    });

// Bind to 0.0.0.0:8080 and start serving webhook callbacks
listener.serve("0.0.0.0:8080").await?;
```

---

## Bot Framework

The bot features require the `bot` feature flag.

```toml
[dependencies]
synaptic-lark = { version = "0.2", features = ["bot"] }
```

### LarkBotClient

Send and reply to messages, and query bot information via the Feishu Bot API.

```rust,ignore
use synaptic::lark::{LarkBotClient, LarkConfig};

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let bot = LarkBotClient::new(config);

// Send a text message to a chat
bot.send_text("oc_xxx", "Hello from Synaptic!").await?;

// Reply to an existing message thread
bot.reply_text("om_xxx", "Got it, processing now...").await?;

// Get information about the bot itself
let info = bot.get_bot_info().await?;
println!("Bot name: {}", info["bot"]["app_name"]);
```

### LarkLongConnListener

Connect to Feishu using a WebSocket long-connection so that no public IP or webhook endpoint is required. Incoming events are deduplicated via an internal LRU cache.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkLongConnListener, MessageHandler};
use synaptic::core::Message;
use async_trait::async_trait;

struct EchoHandler;

#[async_trait]
impl MessageHandler for EchoHandler {
    async fn handle(&self, event: serde_json::Value) -> anyhow::Result<()> {
        let text = event["event"]["message"]["content"].as_str().unwrap_or("");
        println!("Echo: {text}");
        Ok(())
    }
}

let config = LarkConfig::new("cli_xxx", "secret_xxx");
LarkLongConnListener::new(config)
    .with_message_handler(EchoHandler)
    .run()
    .await?;
```
