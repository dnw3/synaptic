# SQL Database Toolkit

`synaptic-sqltoolkit` provides a set of read-only SQL tools for use with LLM agents. Agents can discover available tables, inspect schemas, and run SELECT queries — without any risk of data modification.

## Setup

```toml
[dependencies]
synaptic = { version = "0.2", features = ["sqltoolkit"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
```

## Tools provided

| Tool name | Description |
|-----------|-------------|
| `sql_list_tables` | Lists all tables in the database |
| `sql_describe_table` | Returns column info for a specific table |
| `sql_execute_query` | Executes a SELECT query and returns rows as JSON |

## Quick start

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

## List tables

```rust,ignore
use serde_json::json;

let result = registry.call("sql_list_tables", json!({})).await?;
println!("{}", result);
// {"tables": ["users", "orders", "products"]}
```

## Describe a table

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

## Execute a SELECT query

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

## Use with an agent

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

## Security

All three tools enforce read-only access:

- **`sql_list_tables`** — queries `sqlite_master` (system table, read-only)
- **`sql_describe_table`** — validates the table name against an allowlist (`[a-zA-Z0-9_]` only) to prevent SQL injection; uses `PRAGMA table_info`
- **`sql_execute_query`** — rejects any query that does not start with `SELECT`

```rust,ignore
// This will return an error:
registry.call("sql_execute_query", json!({ "query": "DROP TABLE users" })).await?;
// Err: "Only SELECT queries are allowed for safety."

// Injection attempt rejected:
registry.call("sql_describe_table", json!({ "table_name": "users; DROP TABLE users--" })).await?;
// Err: "Invalid table name: 'users; DROP TABLE users--'..."
```

## Error handling

```rust,ignore
use synaptic::core::SynapticError;

match registry.call("sql_execute_query", json!({ "query": "SELECT 1" })).await {
    Ok(result) => println!("Rows: {}", result["row_count"]),
    Err(SynapticError::Tool(msg)) => eprintln!("SQL error: {msg}"),
    Err(e) => return Err(e.into()),
}
```

## Type mapping

| SQLite type | JSON type |
|-------------|-----------|
| `INTEGER`, `INT`, `BIGINT` | number (integer) |
| `REAL`, `FLOAT`, `DOUBLE` | number (float) |
| `BOOLEAN`, `BOOL` | boolean |
| `TEXT`, `BLOB`, others | string |
| NULL / parse error | `null` |
