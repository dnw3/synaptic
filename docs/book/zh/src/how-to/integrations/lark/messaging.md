# 消息 & Bot

## LarkMessageTool

作为 Agent 工具，向飞书群聊或用户发送消息。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkMessageTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkMessageTool::new(config);

// 发送文本消息
let result = tool.call(json!({
    "action": "send",
    "receive_id_type": "chat_id",
    "receive_id": "oc_xxx",
    "msg_type": "text",
    "content": "来自 Synaptic Agent 的问候！"
})).await?;

println!("消息 ID: {}", result["message_id"]);
```

### 操作说明

| 操作 | 必填字段 | 说明 |
|------|---------|------|
| `send`（默认） | `receive_id_type`, `receive_id`, `msg_type`, `content` | 发送新消息 |
| `update` | `message_id`, `msg_type`, `content` | 更新已有消息 |
| `delete` | `message_id` | 撤回消息 |

### 参数说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `receive_id_type` | 字符串 | `"chat_id"` \| `"user_id"` \| `"email"` \| `"open_id"` |
| `receive_id` | 字符串 | 与 receive_id_type 对应的接收方 ID |
| `msg_type` | 字符串 | `"text"` \| `"post"`（富文本）\| `"interactive"`（卡片） |
| `content` | 字符串 | text 类型为纯文本，post/interactive 类型为 JSON 字符串 |

---

## LarkEventListener

订阅飞书 Webhook 事件，内置 HMAC-SHA256 签名验证和 URL challenge 自动响应。通过事件名称注册类型化的处理函数。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkEventListener};

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let listener = LarkEventListener::new(config)
    .on("im.message.receive_v1", |event| async move {
        let msg = &event["event"]["message"]["content"];
        println!("收到消息: {}", msg);
        Ok(())
    });

// 绑定到 0.0.0.0:8080 并开始提供 Webhook 回调服务
listener.serve("0.0.0.0:8080").await?;
```

---

## Bot 框架

Bot 功能需要开启 `bot` feature。

```toml
[dependencies]
synaptic-lark = { version = "0.2", features = ["bot"] }
```

### LarkBotClient

通过飞书 Bot API 发送消息、回复消息并查询机器人信息。

```rust,ignore
use synaptic::lark::{LarkBotClient, LarkConfig};

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let bot = LarkBotClient::new(config);

// 向群聊发送文本消息
bot.send_text("oc_xxx", "来自 Synaptic 的问候！").await?;

// 回复某条消息所在的会话
bot.reply_text("om_xxx", "收到，正在处理...").await?;

// 获取机器人自身信息
let info = bot.get_bot_info().await?;
println!("机器人名称: {}", info["bot"]["app_name"]);
```

### LarkLongConnListener

通过 WebSocket 长连接接入飞书，无需公网 IP 或 Webhook 域名。内置 LRU 去重缓存，防止重复消费同一事件。

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
