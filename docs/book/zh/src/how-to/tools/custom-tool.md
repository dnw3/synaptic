# 自定义工具

Synaptic 中的每个工具都实现了 `synaptic-core` 中的 `Tool` trait。推荐使用 `#[tool]` 宏来定义工具，它会自动生成结构体、trait 实现和 JSON Schema。

## 使用 `#[tool]` 宏定义工具

`#[tool]` 宏从一个带注解的异步函数生成完整的 `Tool` 实现。函数上的文档注释成为工具描述；参数上的文档注释成为 JSON Schema 描述。

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use serde_json::{json, Value};

/// 获取指定地点的当前天气。
#[tool]
async fn get_weather(
    /// 城市名称
    location: String,
) -> Result<Value, SynapticError> {
    // 实际应用中，在此处调用真实的天气 API
    Ok(json!({
        "location": location,
        "temperature": 22,
        "condition": "sunny"
    }))
}

// `get_weather()` 返回 Arc<dyn Tool>
let tool = get_weather();
assert_eq!(tool.name(), "get_weather");
```

要点：

- 函数上的 `///` 文档注释成为 `description()`，告诉模型这个工具的功能。
- 参数上的 `///` 文档注释成为 JSON Schema 中的字段描述。
- 函数名自动成为 `name()`（也可以用 `#[tool(name = "xxx")]` 覆盖）。
- 宏自动从参数类型生成 JSON Schema（`String` → `"string"`、`i64` → `"integer"` 等）。
- 工厂函数返回 `Arc<dyn Tool>`，可直接注册到 `ToolRegistry`。

## 错误处理

对于工具特定的错误，返回 `SynapticError::Tool(...)`：

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use serde_json::{json, Value};

/// 两数相除。
#[tool]
async fn divide(
    /// 被除数
    a: f64,
    /// 除数
    b: f64,
) -> Result<Value, SynapticError> {
    if b == 0.0 {
        return Err(SynapticError::Tool("除数不能为零".to_string()));
    }
    Ok(json!({"result": a / b}))
}
```

宏自动处理参数的反序列化。如果模型传入了缺失或类型错误的参数，框架会返回相应的错误。

## 注册和使用

工厂函数返回 `Arc<dyn Tool>`，可直接注册到 `ToolRegistry`：

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use synaptic::tools::{ToolRegistry, SerialToolExecutor};
use serde_json::{json, Value};

/// 获取指定地点的当前天气。
#[tool]
async fn get_weather(
    /// 城市名称
    location: String,
) -> Result<Value, SynapticError> {
    Ok(json!({
        "location": location,
        "temperature": 22,
        "condition": "sunny"
    }))
}

let registry = ToolRegistry::new();
registry.register(get_weather())?;

let executor = SerialToolExecutor::new(registry);
let result = executor.execute("get_weather", json!({"location": "Tokyo"})).await?;
// result = {"location": "Tokyo", "temperature": 22, "condition": "sunny"}
```

有关注册和执行的更多信息，请参阅[工具注册表](registry.md)页面。

## 模型的工具定义

宏生成的工具实现了 `as_tool_definition()` 方法，可以自动生成包含完整 JSON Schema 的 `ToolDefinition`。将其附加到 `ChatRequest` 上，告知模型可用的工具：

```rust,ignore
use synaptic::core::{ChatRequest, Message};

let tool = get_weather();
let tool_def = tool.as_tool_definition();

let request = ChatRequest::new(vec![
    Message::human("What is the weather in Tokyo?"),
])
.with_tools(vec![tool_def]);
```

也可以手动构建 `ToolDefinition`，`parameters` 字段遵循 LLM 提供商期望的 JSON Schema 格式。

## 可选参数和默认值

```rust,ignore
use synaptic::macros::tool;
use synaptic::core::SynapticError;

/// 搜索内容。
#[tool]
async fn search(
    /// 搜索查询
    query: String,
    /// 最大结果数（默认 10）
    #[default = 10]
    max_results: i64,
    /// 语言过滤
    language: Option<String>,
) -> Result<String, SynapticError> {
    let lang = language.unwrap_or_else(|| "en".into());
    Ok(format!("搜索 '{}' (最多 {} 条, 语言 {})", query, max_results, lang))
}
```

- `Option<T>` 参数在 schema 中不是 required，缺失时反序列化为 `None`。
- `#[default = value]` 提供编译时默认值，同样不是 required，默认值记录在 schema 的 `"default"` 字段中。

## 使用 `#[field]` 的有状态工具

需要持有状态的工具（数据库连接、API 客户端等）可以使用 `#[field]` 创建对 LLM schema 隐藏的结构体字段：

```rust,ignore
use std::sync::Arc;
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use serde_json::Value;

/// 执行数据库查询。
#[tool]
async fn db_query(
    #[field] pool: Arc<DbPool>,
    /// 要执行的 SQL 查询
    query: String,
) -> Result<Value, SynapticError> {
    let result = pool.execute(&query).await?;
    Ok(serde_json::to_value(result).unwrap())
}

// 工厂函数需要提供 field 参数
let tool = db_query(pool.clone());
```

`#[field]` 参数在构造时提供，对 LLM 完全隐藏。有关 `#[inject]`、`#[default]` 和中间件宏的完整宏参考，请参阅[过程宏](../macros.md)页面。

---

## 手动实现（参考）

如果需要更精细的控制，也可以手动实现 `Tool` trait。`Tool` trait 要求实现三个方法：`name()`、`description()` 和 `call()`：

```rust
use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic::core::{Tool, SynapticError};

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &'static str {
        "get_weather"
    }

    fn description(&self) -> &'static str {
        "Get the current weather for a location"
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let location = args["location"]
            .as_str()
            .unwrap_or("unknown");

        Ok(json!({
            "location": location,
            "temperature": 22,
            "condition": "sunny"
        }))
    }
}
```

手动实现适用于需要自定义 `as_tool_definition()`（例如添加 `extras`）或无法使用宏的场景。大多数情况下推荐使用 `#[tool]` 宏。
