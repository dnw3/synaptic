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

## LarkDocLoader

将飞书文档和知识库页面加载为 Synaptic [`Document`]，可直接用于 RAG 管道。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDocLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");

// 加载指定文档 token
let loader = LarkDocLoader::new(config.clone())
    .with_doc_tokens(vec!["doxcnAbcXxx".to_string()]);

// 或遍历整个 Wiki 空间
let loader = LarkDocLoader::new(config)
    .with_wiki_space_id("spcXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("标题: {}", doc.metadata["title"]);
    println!("URL:  {}", doc.metadata["url"]);
    println!("长度: {} 字符", doc.content.len());
}
```

### 文档 Metadata 字段

| 字段 | 说明 |
|------|------|
| `doc_id` | 飞书文档 token |
| `title` | 文档标题 |
| `source` | `lark:doc:<token>` |
| `url` | 飞书文档直链 |
| `doc_type` | 固定为 `"docx"` |

### 构建器选项

| 方法 | 说明 |
|------|------|
| `with_doc_tokens(tokens)` | 加载指定文档 token 列表 |
| `with_wiki_space_id(id)` | 遍历 Wiki 空间内的所有文档 |

---

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
    "receive_id_type": "chat_id",
    "receive_id": "oc_xxx",
    "msg_type": "text",
    "content": "来自 Synaptic Agent 的问候！"
})).await?;

println!("消息 ID: {}", result["message_id"]);
```

### 参数说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `receive_id_type` | 字符串 | ✅ | `"chat_id"` \| `"user_id"` \| `"email"` \| `"open_id"` |
| `receive_id` | 字符串 | ✅ | 与 receive_id_type 对应的接收方 ID |
| `msg_type` | 字符串 | ✅ | `"text"` \| `"post"`（富文本）\| `"interactive"`（卡片） |
| `content` | 字符串 | ✅ | text 类型为纯文本，post/interactive 类型为 JSON 字符串 |

---

## LarkBitableTool

对飞书多维表格（Bitable）执行查询、新增和更新操作。

```rust,ignore
use synaptic::lark::{LarkBitableTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkBitableTool::new(config);

// 查询记录
let records = tool.call(json!({
    "action": "search",
    "app_token": "bascnXxx",
    "table_id": "tblXxx",
    "filter": { "field": "状态", "value": "待处理" }
})).await?;

// 新建记录
let created = tool.call(json!({
    "action": "create",
    "app_token": "bascnXxx",
    "table_id": "tblXxx",
    "records": [{ "任务": "新事项", "状态": "进行中" }]
})).await?;

// 更新记录
let updated = tool.call(json!({
    "action": "update",
    "app_token": "bascnXxx",
    "table_id": "tblXxx",
    "record_id": "recXxx",
    "fields": { "状态": "完成" }
})).await?;
```

### 操作说明

| 操作 | 额外必填字段 | 说明 |
|------|------------|------|
| `search` | `filter?`（可选）| 查询记录，支持字段过滤 |
| `create` | `records` | 批量新建记录 |
| `update` | `record_id`, `fields` | 更新指定记录的字段 |

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

## LarkBitableLoader

将飞书多维表格中的记录加载为 Synaptic [`Document`]，可用于 RAG 管道或批量数据处理。表格的每一行会生成一个 `Document`，所有字段值存入 metadata。

```rust,ignore
use synaptic::lark::{LarkBitableLoader, LarkConfig};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkBitableLoader::new(config, "bascnXxx", "tblXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("记录 ID: {}", doc.metadata["record_id"]);
    println!("内容: {}", doc.content);
}
```

---

## LarkBitableMemoryStore

将会话历史持久化存储到飞书多维表格，实现多会话、可跨重启的对话记忆。

```rust,ignore
use synaptic::lark::{LarkBitableMemoryStore, LarkConfig};
use synaptic::core::MemoryStore;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let store = LarkBitableMemoryStore::new(config, "bascnXxx", "tblXxx");

// 保存某会话的消息
store.add_messages("session_001", &messages).await?;

// 读取会话历史
let history = store.get_messages("session_001").await?;
```

---

## LarkBitableCheckpointer

将图状态检查点持久化到飞书多维表格，支持长时间运行的 `StateGraph` 中断后恢复执行。需要开启 `checkpointer` feature。

```toml
[dependencies]
synaptic-lark = { version = "0.2", features = ["checkpointer"] }
```

```rust,ignore
use synaptic::lark::{LarkBitableCheckpointer, LarkConfig};
use synaptic::graph::{CompiledGraph, RunnableConfig};

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let checkpointer = LarkBitableCheckpointer::new(config, "bascnXxx", "tblXxx");

// 编译图时传入 checkpointer
let graph = StateGraph::new()
    // ...添加节点和边...
    .compile_with_checkpointer(Box::new(checkpointer));

// 通过相同的 thread_id 恢复上次运行
let run_config = RunnableConfig::default().with_thread_id("thread_001");
let result = graph.invoke_with_config(state, run_config).await?;
```

---

## LarkBitableLlmCache

将 LLM 响应缓存到飞书多维表格，避免对相同提示重复调用 API，从而降低延迟和成本。

```rust,ignore
use synaptic::lark::{LarkBitableLlmCache, LarkConfig};
use synaptic::cache::CachedChatModel;
use synaptic::openai::OpenAiChatModel;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let cache = LarkBitableLlmCache::new(config, "bascnXxx", "tblXxx");

let base_model = OpenAiChatModel::from_env();
let model = CachedChatModel::new(base_model, cache);

// 相同提示第二次调用将从 Bitable 直接返回缓存结果
let response = model.chat(request).await?;
```

---

## LarkSpreadsheetLoader

将飞书电子表格的行加载为 Synaptic [`Document`]。每一行生成一个文档，列标题作为 metadata 的键存储。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkSpreadsheetLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkSpreadsheetLoader::new(config, "shtcnXxx", "0");

let docs = loader.load().await?;
for doc in &docs {
    println!("行内容: {}", doc.content);
    println!("表格 ID: {}", doc.metadata["sheet_id"]);
}
```

---

## LarkWikiLoader

递归加载飞书知识库空间中的所有页面，生成 `Document` 列表。通过 `with_space_id` 指定目标空间，`with_max_depth` 控制递归深度。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkWikiLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkWikiLoader::new(config)
    .with_space_id("spcXxx")
    .with_max_depth(3);

let docs = loader.load().await?;
println!("共加载 {} 个知识库页面", docs.len());
```

---

## LarkDriveLoader

从飞书云盘文件夹加载文件，根据文件类型自动分派到对应的子加载器（文档、电子表格等）进行处理。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDriveLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkDriveLoader::new(config, "fldcnXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("{}: {} 字符", doc.metadata["file_name"], doc.content.len());
}
```

---

## LarkOcrTool

调用飞书 OCR API（`POST /optical_char_recognition/v1/image/basic_recognize`）从图片中提取文本，适用于在 Agent 中处理截图或扫描件。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkOcrTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkOcrTool::new(config);

let result = tool.call(json!({
    "image": "<base64-encoded-image>"
})).await?;

println!("识别文字: {}", result["text"]);
```

---

## LarkTranslateTool

调用飞书翻译 API（`POST /translation/v1/text/translate`）在多种语言之间翻译文本，支持飞书平台提供的所有语言对。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkTranslateTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkTranslateTool::new(config);

let result = tool.call(json!({
    "source_language": "zh",
    "target_language": "en",
    "text": "你好，世界！"
})).await?;

println!("翻译结果: {}", result["translated_text"]);
```

---

## LarkAsrTool

调用飞书语音识别 API（`POST /speech_to_text/v1/speech/file_recognize`）将音频文件转录为文本，接受 base64 编码的音频数据。

```rust,ignore
use synaptic::lark::{LarkAsrTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkAsrTool::new(config);

let result = tool.call(json!({
    "speech": "<base64-encoded-audio>",
    "format": "wav",
    "sample_rate": 16000
})).await?;

println!("转录文本: {}", result["recognition_text"]);
```

---

## LarkDocProcessTool

调用飞书文档 AI API（`POST /document_ai/v1/entity/recognize`）从文档中提取结构化实体，支持表单、发票等多种文档类型，返回结构化键值对。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDocProcessTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkDocProcessTool::new(config);

let result = tool.call(json!({
    "type": "invoice",
    "file_id": "boxcnXxx"
})).await?;

println!("识别实体: {}", result["entities"]);
```

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

## LarkVectorStore

以飞书搜索 API 作为向量存储后端，由飞书平台负责服务端向量化，文档在 Lark 中建立索引后即可通过语义搜索检索。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkVectorStore};
use synaptic::core::VectorStore;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let store = LarkVectorStore::new(config, "data_source_id_xxx");

// 建立文档索引
store.add_documents(docs).await?;

// 语义搜索——向量化由飞书平台处理
let results = store.similarity_search("季度营收", 5).await?;
for doc in &results {
    println!("{}", doc.content);
}
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
