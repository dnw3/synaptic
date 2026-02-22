# Wikipedia

`WikipediaTool` 通过 MediaWiki API 搜索并获取 [Wikipedia](https://www.wikipedia.org/) 文章摘要，无需 API key。

## 设置

```toml
[dependencies]
synaptic = { version = "0.2", features = ["tools"] }
```

## 基本用法

```rust,ignore
use synaptic::tools::WikipediaTool;
use synaptic::core::Tool;
use serde_json::json;

let tool = WikipediaTool::new();

let result = tool.call(json!({ "query": "大语言模型" })).await?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

## 配置

```rust,ignore
// 默认：英文 Wikipedia，最多 3 条结果
let tool = WikipediaTool::new();

// 自定义语言和结果数量
let tool = WikipediaTool::builder()
    .language("zh")        // 中文 Wikipedia
    .max_results(5)
    .build();
```

### 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `language` | `String` | `"en"` | Wikipedia 语言代码（如 `"en"`、`"zh"`、`"de"`） |
| `max_results` | `usize` | `3` | 最多返回文章数 |

## 工具参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `query` | `string` | 是 | 用于搜索 Wikipedia 文章的查询字符串 |

## 响应格式

工具返回文章摘要的 JSON 数组：

```json
{
  "query": "大语言模型",
  "results": [
    {
      "title": "Large language model",
      "url": "https://en.wikipedia.org/wiki/Large_language_model",
      "summary": "A large language model (LLM) is a type of machine learning model...",
      "extract": "A large language model (LLM) is a type of machine learning model designed to understand and generate human language..."
    }
  ]
}
```

| 字段 | 说明 |
|------|------|
| `title` | 文章标题 |
| `url` | Wikipedia 完整 URL |
| `summary` | 简短描述（1~2 句话） |
| `extract` | 文章正文较长摘录 |

## 与 Agent 配合使用

```rust,ignore
use synaptic::tools::WikipediaTool;
use synaptic::core::Tool;
use synaptic::models::OpenAiChatModel;
use synaptic::graph::create_react_agent;
use std::sync::Arc;

let model = Arc::new(OpenAiChatModel::from_env()?);
let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(WikipediaTool::new())];

let agent = create_react_agent(model, tools);
```

## 组合 DuckDuckGo 与 Wikipedia

搭配两个工具可构建更强的研究型 Agent：

```rust,ignore
use synaptic::tools::{DuckDuckGoTool, WikipediaTool};
use synaptic::core::Tool;
use std::sync::Arc;

let tools: Vec<Arc<dyn Tool>> = vec![
    Arc::new(DuckDuckGoTool::new()),
    Arc::new(WikipediaTool::new()),
];
```

## 错误处理

```rust,ignore
use synaptic::core::SynapticError;

match tool.call(json!({ "query": "Rust 编程" })).await {
    Ok(result) => {
        for article in result["results"].as_array().unwrap_or(&vec![]) {
            println!("{}: {}", article["title"], article["summary"]);
        }
    }
    Err(SynapticError::Tool(msg)) => eprintln!("Wikipedia 错误：{msg}"),
    Err(e) => return Err(e.into()),
}
```
