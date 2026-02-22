# Feishu / Lark Integration

The `synaptic-lark` crate integrates Synaptic with the [Feishu/Lark Open Platform](https://open.feishu.cn/), providing document loaders and Agent tools for interacting with Feishu services.

## Setup

```toml
[dependencies]
synaptic = { version = "0.2", features = ["lark"] }
```

Create a custom app at the [Feishu Developer Console](https://open.feishu.cn/app), obtain your **App ID** and **App Secret**, and enable the required scopes (see [Permissions](#permissions) below).

## Configuration

```rust,ignore
use synaptic::lark::LarkConfig;

let config = LarkConfig::new("cli_xxx", "app_secret_xxx");
```

The `tenant_access_token` is fetched and refreshed automatically — tokens are valid for 7,200 seconds and are renewed when fewer than 300 seconds remain.

---

## Using with a ReAct Agent

```rust,ignore
use synaptic::lark::{LarkBitableTool, LarkConfig, LarkMessageTool};
use synaptic::graph::create_react_agent;
use synaptic::openai::OpenAiChatModel;

let model = OpenAiChatModel::from_env();
let config = LarkConfig::new("cli_xxx", "secret_xxx");

let tools: Vec<Box<dyn synaptic::core::Tool>> = vec![
    Box::new(LarkBitableTool::new(config.clone())),
    Box::new(LarkMessageTool::new(config)),
];
let agent = create_react_agent(model, tools);

let result = agent.invoke(
    synaptic::graph::MessageState::from("Check all pending tasks and send a summary to chat oc_xxx"),
).await?;
```

---

## Permissions

Enable the following scopes in the Feishu Developer Console under **Permissions & Scopes**:

| Feature | Required Scope |
|---------|---------------|
| LarkDocLoader (documents) | `docx:document:readonly` |
| LarkDocLoader (Wiki) | `wiki:wiki:readonly` |
| LarkMessageTool | `im:message:send_as_bot` |
| LarkBitableTool (read) | `bitable:app:readonly` |
| LarkBitableTool (write) | `bitable:app` |
| LarkBitableLoader | `bitable:app:readonly` |
| LarkBitableMemoryStore | `bitable:app` |
| LarkBitableCheckpointer | `bitable:app` |
| LarkBitableLlmCache | `bitable:app` |
| LarkSpreadsheetLoader | `sheets:spreadsheet:readonly` |
| LarkWikiLoader | `wiki:wiki:readonly` |
| LarkDriveLoader | `drive:drive:readonly` |
| LarkOcrTool | `optical_char_recognition:image:recognize` |
| LarkTranslateTool | `translation:text:translate` |
| LarkAsrTool | `speech_to_text:speech:recognize` |
| LarkDocProcessTool | `document_ai:entity:recognize` |
| LarkEventListener | (scope depends on subscribed event) |
| LarkVectorStore | `search:data_source:write`, `search:query:execute` |
| LarkBotClient / LarkLongConnListener | `im:message:send_as_bot`, `im:message:receive` |
| LarkContactTool (read users) | `contact:user.base:readonly` |
| LarkContactTool (read departments) | `contact:department.base:readonly` |
| LarkChatTool | `im:chat`, `im:chat.members` |
| LarkSpreadsheetTool | `sheets:spreadsheet` |
| LarkCalendarTool | `calendar:calendar`, `calendar:calendar.event` |
| LarkTaskTool | `task:task` |
