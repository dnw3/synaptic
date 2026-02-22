//! SQL database toolkit for the Synaptic framework.
//!
//! Provides a set of read-only SQL tools for use with LLM agents:
//!
//! - [`ListTablesTool`] — lists all tables in the database
//! - [`DescribeTableTool`] — returns column info for a table
//! - [`ExecuteQueryTool`] — runs a SELECT query and returns results as JSON
//!
//! # Quick start
//!
//! ```rust,ignore
//! use sqlx::sqlite::SqlitePoolOptions;
//! use synaptic_sqltoolkit::SqlToolkit;
//! use synaptic_tools::ToolRegistry;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = SqlitePoolOptions::new()
//!     .connect("sqlite::memory:")
//!     .await?;
//!
//! let toolkit = SqlToolkit::sqlite(pool);
//! let registry = ToolRegistry::new();
//! for tool in toolkit.tools() {
//!     registry.register(tool)?;
//! }
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{Column, Row, SqlitePool, TypeInfo};
use std::sync::Arc;
use synaptic_core::{SynapticError, Tool};

// ---------------------------------------------------------------------------
// SqlToolkit
// ---------------------------------------------------------------------------

/// A toolkit that provides SQL tools for agent use.
pub struct SqlToolkit {
    pool: SqlitePool,
}

impl SqlToolkit {
    /// Create a toolkit backed by a SQLite pool.
    pub fn sqlite(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return the set of tools provided by this toolkit.
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(ListTablesTool {
                pool: self.pool.clone(),
            }),
            Arc::new(DescribeTableTool {
                pool: self.pool.clone(),
            }),
            Arc::new(ExecuteQueryTool {
                pool: self.pool.clone(),
            }),
        ]
    }
}

// ---------------------------------------------------------------------------
// ListTablesTool
// ---------------------------------------------------------------------------

/// Tool that lists all tables in the SQLite database.
pub struct ListTablesTool {
    pool: SqlitePool,
}

#[async_trait]
impl Tool for ListTablesTool {
    fn name(&self) -> &'static str {
        "sql_list_tables"
    }

    fn description(&self) -> &'static str {
        "List all tables available in the SQL database. Returns a JSON array of table names."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {},
            "required": []
        }))
    }

    async fn call(&self, _args: Value) -> Result<Value, SynapticError> {
        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SynapticError::Tool(format!("ListTables error: {e}")))?;

        let tables: Vec<String> = rows.iter().map(|r| r.get::<String, _>("name")).collect();

        Ok(json!({ "tables": tables }))
    }
}

// ---------------------------------------------------------------------------
// DescribeTableTool
// ---------------------------------------------------------------------------

/// Tool that returns the schema of a specific table.
pub struct DescribeTableTool {
    pool: SqlitePool,
}

/// Validates that an identifier contains only safe characters (alphanumeric + underscore).
fn is_safe_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

#[async_trait]
impl Tool for DescribeTableTool {
    fn name(&self) -> &'static str {
        "sql_describe_table"
    }

    fn description(&self) -> &'static str {
        "Describe the schema of a SQL table. Returns column names, types, and constraints."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "table_name": {
                    "type": "string",
                    "description": "The name of the table to describe"
                }
            },
            "required": ["table_name"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let table_name = args["table_name"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'table_name' parameter".to_string()))?;

        if !is_safe_identifier(table_name) {
            return Err(SynapticError::Tool(format!(
                "Invalid table name: '{table_name}'. Only alphanumeric characters and underscores are allowed."
            )));
        }

        let sql = format!("PRAGMA table_info({table_name})");
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SynapticError::Tool(format!("DescribeTable error: {e}")))?;

        if rows.is_empty() {
            return Err(SynapticError::Tool(format!(
                "Table '{table_name}' does not exist."
            )));
        }

        let columns: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "cid": r.get::<i64, _>("cid"),
                    "name": r.get::<String, _>("name"),
                    "type": r.get::<String, _>("type"),
                    "not_null": r.get::<bool, _>("notnull"),
                    "primary_key": r.get::<i64, _>("pk") > 0,
                })
            })
            .collect();

        Ok(json!({
            "table": table_name,
            "columns": columns,
        }))
    }
}

// ---------------------------------------------------------------------------
// ExecuteQueryTool
// ---------------------------------------------------------------------------

/// Tool that executes a read-only SQL SELECT query.
pub struct ExecuteQueryTool {
    pool: SqlitePool,
}

#[async_trait]
impl Tool for ExecuteQueryTool {
    fn name(&self) -> &'static str {
        "sql_execute_query"
    }

    fn description(&self) -> &'static str {
        "Execute a read-only SQL SELECT query and return results as JSON. \
         Only SELECT statements are allowed for safety. \
         Returns an array of objects, one per row."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "A SQL SELECT query to execute. Must start with SELECT."
                }
            },
            "required": ["query"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'query' parameter".to_string()))?;

        // Safety: only allow SELECT statements
        let trimmed = query.trim_start().to_uppercase();
        if !trimmed.starts_with("SELECT") {
            return Err(SynapticError::Tool(
                "Only SELECT queries are allowed for safety.".to_string(),
            ));
        }

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SynapticError::Tool(format!("Query execution error: {e}")))?;

        let results: Vec<Value> = rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let name = col.name().to_string();
                    let type_name = col.type_info().name();
                    let val: Value = match type_name {
                        "INTEGER" | "INT" | "BIGINT" => {
                            if let Ok(v) = row.try_get::<i64, _>(i) {
                                json!(v)
                            } else {
                                Value::Null
                            }
                        }
                        "REAL" | "FLOAT" | "DOUBLE" => {
                            if let Ok(v) = row.try_get::<f64, _>(i) {
                                json!(v)
                            } else {
                                Value::Null
                            }
                        }
                        "BOOLEAN" | "BOOL" => {
                            if let Ok(v) = row.try_get::<bool, _>(i) {
                                json!(v)
                            } else {
                                Value::Null
                            }
                        }
                        _ => {
                            // Default to string for TEXT, BLOB, NULL, etc.
                            if let Ok(v) = row.try_get::<String, _>(i) {
                                json!(v)
                            } else {
                                Value::Null
                            }
                        }
                    };
                    obj.insert(name, val);
                }
                Value::Object(obj)
            })
            .collect();

        Ok(json!({
            "rows": results,
            "row_count": results.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite")
    }

    #[test]
    fn is_safe_identifier_allows_valid() {
        assert!(is_safe_identifier("users"));
        assert!(is_safe_identifier("my_table_123"));
        assert!(is_safe_identifier("ABC"));
    }

    #[test]
    fn is_safe_identifier_blocks_injection() {
        assert!(!is_safe_identifier("users; DROP TABLE users--"));
        assert!(!is_safe_identifier("users--"));
        assert!(!is_safe_identifier(""));
        assert!(!is_safe_identifier("tab le"));
    }

    #[tokio::test]
    async fn list_tables_empty_db() {
        let pool = test_pool().await;
        let tool = ListTablesTool { pool };
        let result = tool.call(json!({})).await.unwrap();
        assert_eq!(result["tables"], json!([]));
    }

    #[tokio::test]
    async fn list_tables_with_data() {
        let pool = test_pool().await;
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        let tool = ListTablesTool { pool: pool.clone() };
        let result = tool.call(json!({})).await.unwrap();
        let tables = result["tables"].as_array().unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0], "users");
    }

    #[tokio::test]
    async fn describe_table() {
        let pool = test_pool().await;
        sqlx::query(
            "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT NOT NULL, price REAL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let tool = DescribeTableTool { pool: pool.clone() };
        let result = tool.call(json!({"table_name": "products"})).await.unwrap();
        let cols = result["columns"].as_array().unwrap();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0]["name"], "id");
        assert_eq!(cols[0]["primary_key"], true);
    }

    #[tokio::test]
    async fn execute_select_query() {
        let pool = test_pool().await;
        sqlx::query("CREATE TABLE items (id INTEGER, label TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items VALUES (1, 'alpha'), (2, 'beta')")
            .execute(&pool)
            .await
            .unwrap();
        let tool = ExecuteQueryTool { pool: pool.clone() };
        let result = tool
            .call(json!({"query": "SELECT id, label FROM items ORDER BY id"}))
            .await
            .unwrap();
        assert_eq!(result["row_count"], 2);
        assert_eq!(result["rows"][0]["label"], "alpha");
    }

    #[tokio::test]
    async fn execute_non_select_rejected() {
        let pool = test_pool().await;
        let tool = ExecuteQueryTool { pool };
        let err = tool
            .call(json!({"query": "DROP TABLE users"}))
            .await
            .unwrap_err();
        assert!(matches!(err, SynapticError::Tool(_)));
    }
}
