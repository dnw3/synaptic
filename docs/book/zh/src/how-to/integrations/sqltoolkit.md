# SQL 数据库工具包

`synaptic-sqltoolkit` 为 LLM Agent 提供一组只读 SQL 工具。Agent 可发现可用表、查看表结构、执行 SELECT 查询，而不会造成任何数据修改风险。

## 设置

```toml
[dependencies]
synaptic = { version = "0.2", features = ["sqltoolkit"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
```

## 提供的工具

| 工具名称 | 说明 |
|---------|------|
| `sql_list_tables` | 列出数据库中所有表 |
| `sql_describe_table` | 返回指定表的列信息 |
| `sql_execute_query` | 执行 SELECT 查询并以 JSON 格式返回结果 |

## 快速开始

```rust,ignore
use sqlx::sqlite::SqlitePoolOptions;
use synaptic::sqltoolkit::SqlToolkit;
use synaptic::tools::ToolRegistry;
use std::sync::Arc;

let pool = SqlitePoolOptions::new()
    .connect("sqlite::memory:")
    .await?;

let toolkit = SqlToolkit::sqlite(pool);

let registry = ToolRegistry::new();
for tool in toolkit.tools() {
    registry.register(tool)?;
}
```

## 列出所有表

```rust,ignore
use serde_json::json;

let result = registry.call("sql_list_tables", json!({})).await?;
println!("{}", result);
// {"tables": ["users", "orders", "products"]}
```

## 查看表结构

```rust,ignore
let result = registry
    .call("sql_describe_table", json!({ "table_name": "users" }))
    .await?;
println!("{}", serde_json::to_string_pretty(&result)?);
// {
//   "table": "users",
//   "columns": [
//     { "cid": 0, "name": "id", "type": "INTEGER", "not_null": true, "primary_key": true },
//     { "cid": 1, "name": "email", "type": "TEXT", "not_null": true, "primary_key": false }
//   ]
// }
```

## 执行 SELECT 查询

```rust,ignore
let result = registry
    .call(
        "sql_execute_query",
        json!({ "query": "SELECT id, email FROM users LIMIT 10" }),
    )
    .await?;
println!("{}", serde_json::to_string_pretty(&result)?);
// {
//   "rows": [
//     { "id": 1, "email": "alice@example.com" },
//     { "id": 2, "email": "bob@example.com" }
//   ],
//   "row_count": 2
// }
```

## 与 Agent 配合使用

```rust,ignore
use sqlx::sqlite::SqlitePoolOptions;
use synaptic::sqltoolkit::SqlToolkit;
use synaptic::models::OpenAiChatModel;
use synaptic::graph::create_react_agent;
use synaptic::core::Tool;
use std::sync::Arc;

let pool = SqlitePoolOptions::new()
    .connect("sqlite:./mydb.sqlite")
    .await?;

let model = Arc::new(OpenAiChatModel::from_env()?);
let tools: Vec<Arc<dyn Tool>> = SqlToolkit::sqlite(pool).tools();

let agent = create_react_agent(model, tools);
```

## 安全性

三个工具均强制只读访问：

- **`sql_list_tables`** — 查询 `sqlite_master`（系统表，只读）
- **`sql_describe_table`** — 对表名进行安全校验（仅允许 `[a-zA-Z0-9_]`），防止 SQL 注入；使用 `PRAGMA table_info`
- **`sql_execute_query`** — 拒绝任何不以 `SELECT` 开头的查询

```rust,ignore
// 以下调用会返回错误：
registry.call("sql_execute_query", json!({ "query": "DROP TABLE users" })).await?;
// Err: "Only SELECT queries are allowed for safety."

// 注入尝试被拒绝：
registry.call("sql_describe_table", json!({ "table_name": "users; DROP TABLE users--" })).await?;
// Err: "Invalid table name: 'users; DROP TABLE users--'..."
```

## 错误处理

```rust,ignore
use synaptic::core::SynapticError;

match registry.call("sql_execute_query", json!({ "query": "SELECT 1" })).await {
    Ok(result) => println!("行数：{}", result["row_count"]),
    Err(SynapticError::Tool(msg)) => eprintln!("SQL 错误：{msg}"),
    Err(e) => return Err(e.into()),
}
```

## 类型映射

| SQLite 类型 | JSON 类型 |
|------------|---------|
| `INTEGER`、`INT`、`BIGINT` | 数字（整数） |
| `REAL`、`FLOAT`、`DOUBLE` | 数字（浮点） |
| `BOOLEAN`、`BOOL` | 布尔值 |
| `TEXT`、`BLOB` 及其他 | 字符串 |
| NULL / 解析错误 | `null` |
