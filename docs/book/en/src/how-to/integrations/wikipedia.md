# Wikipedia

`WikipediaTool` searches and retrieves article summaries from [Wikipedia](https://www.wikipedia.org/) using the MediaWiki API. No API key is required.

## Setup

```toml
[dependencies]
synaptic = { version = "0.2", features = ["tools"] }
```

## Basic usage

```rust,ignore
use synaptic::tools::WikipediaTool;
use synaptic::core::Tool;
use serde_json::json;

let tool = WikipediaTool::new();

let result = tool.call(json!({ "query": "Large language model" })).await?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

## Configuration

```rust,ignore
// Default: English Wikipedia, up to 3 results
let tool = WikipediaTool::new();

// Custom language and result count
let tool = WikipediaTool::builder()
    .language("de")          // German Wikipedia
    .max_results(5)
    .build();
```

### Configuration reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String` | `"en"` | Wikipedia language code (e.g. `"en"`, `"zh"`, `"de"`) |
| `max_results` | `usize` | `3` | Maximum number of articles to return |

## Tool parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | `string` | yes | Search query to find Wikipedia articles |

## Response format

The tool returns a JSON array of article summaries:

```json
{
  "query": "large language model",
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

| Field | Description |
|-------|-------------|
| `title` | Article title |
| `url` | Full Wikipedia URL |
| `summary` | Short description (1–2 sentences) |
| `extract` | Longer text extract from the article |

## Use with an agent

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

## Combining DuckDuckGo and Wikipedia

For richer research agents, combine both tools:

```rust,ignore
use synaptic::tools::{DuckDuckGoTool, WikipediaTool};
use synaptic::core::Tool;
use std::sync::Arc;

let tools: Vec<Arc<dyn Tool>> = vec![
    Arc::new(DuckDuckGoTool::new()),
    Arc::new(WikipediaTool::new()),
];
```

## Error handling

```rust,ignore
use synaptic::core::SynapticError;

match tool.call(json!({ "query": "Rust programming" })).await {
    Ok(result) => {
        for article in result["results"].as_array().unwrap_or(&vec![]) {
            println!("{}: {}", article["title"], article["summary"]);
        }
    }
    Err(SynapticError::Tool(msg)) => eprintln!("Wikipedia error: {msg}"),
    Err(e) => return Err(e.into()),
}
```
