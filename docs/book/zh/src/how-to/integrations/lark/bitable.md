# 多维表格 Bitable

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
| `delete` | `record_id` | 删除单条记录 |
| `batch_update` | `records` | 批量更新多条记录 |
| `batch_delete` | `record_ids` | 批量删除多条记录 |
| `list_tables` | — | 列出应用中的所有数据表 |
| `create_table` | `table_name` | 新建数据表 |
| `delete_table` | `table_id` | 删除数据表 |
| `list_fields` | `table_id` | 列出数据表中的所有字段 |
| `create_field` | `table_id`, `field_name`, `field_type` | 新增字段 |
| `update_field` | `table_id`, `field_id`, `field_name` | 重命名字段 |
| `delete_field` | `table_id`, `field_id` | 删除字段 |

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
