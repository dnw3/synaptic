# DuckDuckGo 搜索

`DuckDuckGoTool` 通过 [DuckDuckGo Instant Answer API](https://duckduckgo.com/) 提供免费网络搜索功能，无需 API key 或账号。

## 设置

```toml
[dependencies]
synaptic = { version = "0.2", features = ["tools"] }
```

## 基本用法

```rust,ignore
use synaptic::tools::DuckDuckGoTool;
use synaptic::core::Tool;
use serde_json::json;

let tool = DuckDuckGoTool::new();

let result = tool.call(json!({ "query": "Rust 编程语言" })).await?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

## 配置

```rust,ignore
// 默认：最多返回 5 条结果
let tool = DuckDuckGoTool::new();

// 自定义结果数量
let tool = DuckDuckGoTool::with_max_results(10);
```

### 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_results` | `usize` | `5` | 最多返回结果数 |

## 工具参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `query` | `string` | 是 | 搜索查询字符串 |

## 响应格式

工具返回包含 `query` 和 `results` 字段的 JSON 对象：

```json
{
  "query": "Rust 编程语言",
  "results": [
    {
      "type": "abstract",
      "title": "Rust (programming language)",
      "url": "https://en.wikipedia.org/wiki/Rust_(programming_language)",
      "text": "Rust 是一种多范式、通用编程语言..."
    },
    {
      "type": "related",
      "title": "Cargo (Rust)",
      "url": "https://en.wikipedia.org/wiki/Cargo_(Rust)",
      "text": "Cargo 是 Rust 的包管理器。"
    }
  ]
}
```

结果类型：
- `abstract` — DuckDuckGo 的即时答案摘要（来自 Wikipedia 或精选来源）
- `answer` — 计算或直接答案（如单位换算、定义等）
- `related` — 来自 DuckDuckGo 主题图谱的相关话题

## 与 Agent 配合使用

```rust,ignore
use synaptic::tools::{DuckDuckGoTool, ToolRegistry};
use synaptic::models::OpenAiChatModel;
use synaptic::graph::create_react_agent;
use std::sync::Arc;

let model = Arc::new(OpenAiChatModel::from_env()?);
let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(DuckDuckGoTool::new())];

let agent = create_react_agent(model, tools);
let result = agent.invoke(/* state */).await?;
```

## 注册到 ToolRegistry

```rust,ignore
use synaptic::tools::{DuckDuckGoTool, ToolRegistry};
use std::sync::Arc;

let registry = ToolRegistry::new();
registry.register(Arc::new(DuckDuckGoTool::new()))?;

// 通过 registry 调用
let result = registry.call("duckduckgo_search", json!({ "query": "异步 Rust" })).await?;
```

## 错误处理

```rust,ignore
use synaptic::core::SynapticError;

match tool.call(json!({ "query": "Rust" })).await {
    Ok(result) => println!("结果数：{}", result["results"].as_array().unwrap().len()),
    Err(SynapticError::Tool(msg)) => eprintln!("搜索错误：{msg}"),
    Err(e) => return Err(e.into()),
}
```

## 使用限制

DuckDuckGo Instant Answer API 适合获取简明答案和相关话题，并非完整的网页搜索结果列表。如需更全面的搜索结果，请考虑使用 [Tavily](./tavily.md) 集成。
