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

## LarkDocLoader

Load Feishu documents and Wiki pages into Synaptic [`Document`]s for RAG pipelines.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDocLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");

// Load specific document tokens
let loader = LarkDocLoader::new(config.clone())
    .with_doc_tokens(vec!["doxcnAbcXxx".to_string()]);

// Or traverse an entire Wiki space
let loader = LarkDocLoader::new(config)
    .with_wiki_space_id("spcXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("Title: {}", doc.metadata["title"]);
    println!("URL:   {}", doc.metadata["url"]);
    println!("Length: {} chars", doc.content.len());
}
```

### Document Metadata

Each document includes:

| Field | Description |
|-------|-------------|
| `doc_id` | The Feishu document token |
| `title` | Document title |
| `source` | `lark:doc:<token>` |
| `url` | Direct Feishu document URL |
| `doc_type` | Always `"docx"` |

### Builder Options

| Method | Description |
|--------|-------------|
| `with_doc_tokens(tokens)` | Load specific document tokens |
| `with_wiki_space_id(id)` | Traverse all docs in a Wiki space |

---

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
    "receive_id_type": "chat_id",
    "receive_id": "oc_xxx",
    "msg_type": "text",
    "content": "Hello from Synaptic Agent!"
})).await?;

println!("Sent message ID: {}", result["message_id"]);
```

### Arguments

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `receive_id_type` | string | ✅ | `"chat_id"` \| `"user_id"` \| `"email"` \| `"open_id"` |
| `receive_id` | string | ✅ | The receiver ID matching the type |
| `msg_type` | string | ✅ | `"text"` \| `"post"` (rich text) \| `"interactive"` (card) |
| `content` | string | ✅ | Plain string for text; JSON string for post/interactive |

---

## LarkBitableTool

Search, create, and update records in a Feishu Bitable (multi-dimensional table).

```rust,ignore
use synaptic::lark::{LarkBitableTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkBitableTool::new(config);

// Search records
let records = tool.call(json!({
    "action": "search",
    "app_token": "bascnXxx",
    "table_id": "tblXxx",
    "filter": { "field": "Status", "value": "Pending" }
})).await?;

// Create records
let created = tool.call(json!({
    "action": "create",
    "app_token": "bascnXxx",
    "table_id": "tblXxx",
    "records": [{ "Task": "New item", "Status": "Open" }]
})).await?;

// Update a record
let updated = tool.call(json!({
    "action": "update",
    "app_token": "bascnXxx",
    "table_id": "tblXxx",
    "record_id": "recXxx",
    "fields": { "Status": "Done" }
})).await?;
```

### Actions

| Action | Required extra fields | Description |
|--------|----------------------|-------------|
| `search` | `filter?` | Query records (optional field+value filter) |
| `create` | `records` | Create one or more records |
| `update` | `record_id`, `fields` | Update fields on an existing record |

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

## LarkBitableLoader

Load Bitable records as Synaptic [`Document`]s for use in RAG pipelines or batch processing. Each row in the table becomes one `Document` with all field values stored in metadata.

```rust,ignore
use synaptic::lark::{LarkBitableLoader, LarkConfig};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkBitableLoader::new(config, "bascnXxx", "tblXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("Record ID: {}", doc.metadata["record_id"]);
    println!("Content: {}", doc.content);
}
```

---

## LarkBitableMemoryStore

Store and retrieve chat history in a Feishu Bitable table, enabling persistent multi-session conversation memory backed by Bitable.

```rust,ignore
use synaptic::lark::{LarkBitableMemoryStore, LarkConfig};
use synaptic::core::MemoryStore;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let store = LarkBitableMemoryStore::new(config, "bascnXxx", "tblXxx");

// Save messages for a session
store.add_messages("session_001", &messages).await?;

// Retrieve conversation history
let history = store.get_messages("session_001").await?;
```

---

## LarkBitableCheckpointer

Persist graph state checkpoints in a Feishu Bitable table, allowing long-running `StateGraph` executions to be interrupted and resumed. Requires the `checkpointer` feature.

```toml
[dependencies]
synaptic-lark = { version = "0.2", features = ["checkpointer"] }
```

```rust,ignore
use synaptic::lark::{LarkBitableCheckpointer, LarkConfig};
use synaptic::graph::{CompiledGraph, RunnableConfig};

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let checkpointer = LarkBitableCheckpointer::new(config, "bascnXxx", "tblXxx");

// Pass the checkpointer when compiling the graph
let graph = StateGraph::new()
    // ...add nodes and edges...
    .compile_with_checkpointer(Box::new(checkpointer));

// Resume a prior run by supplying the same thread_id
let run_config = RunnableConfig::default().with_thread_id("thread_001");
let result = graph.invoke_with_config(state, run_config).await?;
```

---

## LarkBitableLlmCache

Cache LLM responses in a Feishu Bitable table to avoid redundant API calls and reduce latency for repeated prompts.

```rust,ignore
use synaptic::lark::{LarkBitableLlmCache, LarkConfig};
use synaptic::cache::CachedChatModel;
use synaptic::openai::OpenAiChatModel;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let cache = LarkBitableLlmCache::new(config, "bascnXxx", "tblXxx");

let base_model = OpenAiChatModel::from_env();
let model = CachedChatModel::new(base_model, cache);

// Identical prompts are served from Bitable on the second call
let response = model.chat(request).await?;
```

---

## LarkSpreadsheetLoader

Load rows from a Feishu spreadsheet as Synaptic [`Document`]s. Each row becomes one document; column headers are stored as metadata keys.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkSpreadsheetLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkSpreadsheetLoader::new(config, "shtcnXxx", "0");

let docs = loader.load().await?;
for doc in &docs {
    println!("Row: {}", doc.content);
    println!("Sheet: {}", doc.metadata["sheet_id"]);
}
```

---

## LarkWikiLoader

Recursively load all pages from a Feishu Wiki space as `Document`s. The `with_space_id` and `with_max_depth` builder methods control which space is traversed and how deep to recurse.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkWikiLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkWikiLoader::new(config)
    .with_space_id("spcXxx")
    .with_max_depth(3);

let docs = loader.load().await?;
println!("Loaded {} Wiki pages", docs.len());
```

---

## LarkDriveLoader

Load files from a Feishu Drive folder, automatically dispatching to the appropriate sub-loader (doc, spreadsheet, etc.) based on file type.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDriveLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkDriveLoader::new(config, "fldcnXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("{}: {} chars", doc.metadata["file_name"], doc.content.len());
}
```

---

## LarkOcrTool

Extract text from images using the Feishu OCR API (`POST /optical_char_recognition/v1/image/basic_recognize`). Useful for processing screenshots or scanned documents inside an agent.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkOcrTool};
use synaptic::core::Tool;
use serde_json::json;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let tool = LarkOcrTool::new(config);

let result = tool.call(json!({
    "image": "<base64-encoded-image>"
})).await?;

println!("Recognized text: {}", result["text"]);
```

---

## LarkTranslateTool

Translate text between languages using the Feishu Translation API (`POST /translation/v1/text/translate`). Supports all language pairs offered by the Feishu platform.

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

println!("Translation: {}", result["translated_text"]);
```

---

## LarkAsrTool

Transcribe audio files to text using the Feishu Speech-to-Text API (`POST /speech_to_text/v1/speech/file_recognize`). Accepts a base64-encoded audio file.

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

println!("Transcript: {}", result["recognition_text"]);
```

---

## LarkDocProcessTool

Extract structured entities from documents using the Feishu Document AI API (`POST /document_ai/v1/entity/recognize`). Returns structured key-value pairs from forms, invoices, and other document types.

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

println!("Entities: {}", result["entities"]);
```

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

## LarkVectorStore

Store and search vectors using the Feishu Search API as the backend. Feishu handles embedding on the server side; your documents are indexed in Lark and retrieved via semantic search.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkVectorStore};
use synaptic::core::VectorStore;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let store = LarkVectorStore::new(config, "data_source_id_xxx");

// Index documents
store.add_documents(docs).await?;

// Semantic search — embedding is handled by the Feishu platform
let results = store.similarity_search("quarterly earnings", 5).await?;
for doc in &results {
    println!("{}", doc.content);
}
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
