# 飞书 / Lark 集成

`synaptic-lark` crate 将 Synaptic 与[飞书开放平台](https://open.feishu.cn/)深度集成，提供文档加载器和 Agent 工具，用于与飞书各类服务进行交互。

## 安装

```toml
[dependencies]
synaptic = { version = "0.2", features = ["lark"] }
```

在[飞书开发者后台](https://open.feishu.cn/app)创建自定义应用，获取 **App ID** 和 **App Secret**，并开启所需权限（详见[权限说明](#权限说明)）。

## 配置

```rust,ignore
use synaptic::lark::LarkConfig;

let config = LarkConfig::new("cli_xxx", "app_secret_xxx");
```

`tenant_access_token` 会自动获取和刷新——token 有效期为 7,200 秒，剩余不足 300 秒时自动续期。

---

## 与 ReAct Agent 结合

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
    synaptic::graph::MessageState::from("查询所有待处理任务并将摘要发送到群聊 oc_xxx"),
).await?;
```

---

## 权限说明

在飞书开发者后台的**权限与范围**页面开启以下权限：

| 功能 | 所需权限 |
|------|---------|
| LarkDocLoader（文档） | `docx:document:readonly` |
| LarkDocLoader（知识库） | `wiki:wiki:readonly` |
| LarkMessageTool | `im:message:send_as_bot` |
| LarkBitableTool（只读） | `bitable:app:readonly` |
| LarkBitableTool（读写） | `bitable:app` |
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
| LarkEventListener | （取决于订阅的事件类型） |
| LarkVectorStore | `search:data_source:write`、`search:query:execute` |
| LarkBotClient / LarkLongConnListener | `im:message:send_as_bot`、`im:message:receive` |
| LarkContactTool（查询用户） | `contact:user.base:readonly` |
| LarkContactTool（查询部门） | `contact:department.base:readonly` |
| LarkChatTool | `im:chat`、`im:chat.members` |
| LarkSpreadsheetTool | `sheets:spreadsheet` |
| LarkCalendarTool | `calendar:calendar`、`calendar:calendar.event` |
| LarkTaskTool | `task:task` |
