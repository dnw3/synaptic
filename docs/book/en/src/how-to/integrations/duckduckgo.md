# DuckDuckGo Search

`DuckDuckGoTool` provides free web search capabilities using the [DuckDuckGo Instant Answer API](https://duckduckgo.com/). No API key or account is required.

## Setup

```toml
[dependencies]
synaptic = { version = "0.2", features = ["tools"] }
```

## Basic usage

```rust,ignore
use synaptic::tools::DuckDuckGoTool;
use synaptic::core::Tool;
use serde_json::json;

let tool = DuckDuckGoTool::new();

let result = tool.call(json!({ "query": "Rust programming language" })).await?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

## Configuration

```rust,ignore
// Default: returns up to 5 results
let tool = DuckDuckGoTool::new();

// Custom result count
let tool = DuckDuckGoTool::with_max_results(10);
```

### Configuration reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_results` | `usize` | `5` | Maximum number of results to return |

## Tool parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | `string` | yes | The search query string |

## Response format

The tool returns a JSON object with `query` and `results` fields:

```json
{
  "query": "Rust programming language",
  "results": [
    {
      "type": "abstract",
      "title": "Rust (programming language)",
      "url": "https://en.wikipedia.org/wiki/Rust_(programming_language)",
      "text": "Rust is a multi-paradigm, general-purpose programming language..."
    },
    {
      "type": "related",
      "title": "Cargo (Rust)",
      "url": "https://en.wikipedia.org/wiki/Cargo_(Rust)",
      "text": "Cargo is the Rust package manager."
    }
  ]
}
```

Result types:
- `abstract` — DuckDuckGo's instant answer abstract (from Wikipedia or curated sources)
- `answer` — Computed or direct answer (e.g., conversions, definitions)
- `related` — Related topics from DuckDuckGo's topic graph

## Use with an agent

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

## Use in a tool registry

```rust,ignore
use synaptic::tools::{DuckDuckGoTool, ToolRegistry};
use std::sync::Arc;

let registry = ToolRegistry::new();
registry.register(Arc::new(DuckDuckGoTool::new()))?;

// Execute via registry
let result = registry.call("duckduckgo_search", json!({ "query": "async Rust" })).await?;
```

## Error handling

```rust,ignore
use synaptic::core::SynapticError;

match tool.call(json!({ "query": "Rust" })).await {
    Ok(result) => println!("Results: {}", result["results"].as_array().unwrap().len()),
    Err(SynapticError::Tool(msg)) => eprintln!("Search error: {msg}"),
    Err(e) => return Err(e.into()),
}
```

## Limitations

The DuckDuckGo Instant Answer API is optimized for concise answers and related topics rather than full web search result lists. For comprehensive search result pages, consider using the [Tavily](./tavily.md) integration.
