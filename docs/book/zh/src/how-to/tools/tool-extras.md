# Tool Definition Extras

`ToolDefinition` 上的 `extras` 字段携带提供商特定的参数，这些参数不在标准的 name/description/parameters schema 范围内，例如 Anthropic 的 `cache_control` 或你的提供商适配器需要的任何自定义元数据。

## `extras` 字段

```rust,ignore
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    /// Provider-specific parameters (e.g., Anthropic's `cache_control`).
    pub extras: Option<HashMap<String, Value>>,
}
```

当 `extras` 为 `None`（默认值）时，不会序列化任何额外字段。提供商适配器在构建请求时会检查 `extras`，并将识别的键映射到提供商的传输格式中。

## 在 Tool Definition 上设置 Extras

通过直接填充字段来构建带有 extras 的 `ToolDefinition`：

```rust,ignore
use std::collections::HashMap;
use serde_json::{json, Value};
use synaptic::core::ToolDefinition;

let mut extras = HashMap::new();
extras.insert("cache_control".to_string(), json!({"type": "ephemeral"}));

let tool_def = ToolDefinition {
    name: "search".to_string(),
    description: "Search the web".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" }
        },
        "required": ["query"]
    }),
    extras: Some(extras),
};
```

## 常见用例

**Anthropic prompt caching** -- Anthropic 在工具定义上支持 `cache_control` 字段，用于对不常变化的工具 schema 启用 prompt 缓存：

```rust,ignore
let mut extras = HashMap::new();
extras.insert("cache_control".to_string(), json!({"type": "ephemeral"}));

let def = ToolDefinition {
    name: "lookup".to_string(),
    description: "Look up a record".to_string(),
    parameters: json!({"type": "object", "properties": {}}),
    extras: Some(extras),
};
```

**自定义元数据** -- 你可以为自己的适配器逻辑附加任意键值对：

```rust,ignore
let mut extras = HashMap::new();
extras.insert("priority".to_string(), json!("high"));
extras.insert("timeout_ms".to_string(), json!(5000));

let def = ToolDefinition {
    name: "deploy".to_string(),
    description: "Deploy the service".to_string(),
    parameters: json!({"type": "object", "properties": {}}),
    extras: Some(extras),
};
```

## 在宏工具中使用 Extras

`#[tool]` 宏目前不直接支持设置 `extras`。推荐的做法是先用宏定义工具，然后通过 `as_tool_definition()` 获取定义，再手动修改 `extras` 字段：

```rust,ignore
use std::collections::HashMap;
use synaptic::macros::tool;
use synaptic::core::SynapticError;
use serde_json::{json, Value};

/// 搜索网页内容。
#[tool]
async fn search(
    /// 搜索查询
    query: String,
) -> Result<Value, SynapticError> {
    Ok(json!({"results": []}))
}

// 获取宏生成的工具定义
let tool = search();
let mut tool_def = tool.as_tool_definition();

// 手动添加 extras（例如 Anthropic prompt 缓存）
let mut extras = HashMap::new();
extras.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
tool_def.extras = Some(extras);

// 将修改后的 tool_def 附加到 ChatRequest
```

> **注意：** `#[tool]` 宏不支持在属性中直接指定 `extras`。如果需要在 `as_tool_definition()` 中自动返回 extras，请使用手动 `impl Tool` 的方式（参见[自定义工具](custom-tool.md)中的手动实现部分）。
