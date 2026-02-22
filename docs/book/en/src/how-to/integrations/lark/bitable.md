# Bitable

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
| `delete` | `record_id` | Delete a single record |
| `batch_update` | `records` | Update multiple records in one call |
| `batch_delete` | `record_ids` | Delete multiple records |
| `list_tables` | — | List all tables in the app |
| `create_table` | `table_name` | Create a new table |
| `delete_table` | `table_id` | Delete a table |
| `list_fields` | `table_id` | List all fields in a table |
| `create_field` | `table_id`, `field_name`, `field_type` | Add a field |
| `update_field` | `table_id`, `field_id`, `field_name` | Rename a field |
| `delete_field` | `table_id`, `field_id` | Delete a field |

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
