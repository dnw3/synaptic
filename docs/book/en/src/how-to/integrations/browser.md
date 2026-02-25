# Browser Automation

CDP (Chrome DevTools Protocol)-based browser tools for Synaptic agents. This crate provides `Tool` implementations that allow agents to navigate web pages, take screenshots, and evaluate JavaScript in a browser session.

## Setup

Add the `browser` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.3", features = ["browser"] }
```

You need a running Chrome or Chromium instance with the DevTools Protocol enabled:

```bash
google-chrome --remote-debugging-port=9222 --headless
```

The browser tools connect to this debug port to issue commands.

## Available Tools

The crate provides three tools, all registered through the `browser_tools()` helper:

```rust,ignore
use synaptic::browser::{BrowserConfig, browser_tools};

let config = BrowserConfig::default();
let tools = browser_tools(&config);
// Returns: [NavigateTool, ScreenshotTool, EvalJsTool]
```

### NavigateTool

Navigates the browser to a specified URL.

- **Tool name:** `browser_navigate`
- **Parameters:** `{ "url": "https://example.com" }` (required)
- **Returns:** Confirmation string on success

### ScreenshotTool

Takes a screenshot of the current browser page.

- **Tool name:** `browser_screenshot`
- **Parameters:** None required
- **Returns:** Base64-encoded PNG image data

### EvalJsTool

Evaluates a JavaScript expression in the current page context.

- **Tool name:** `browser_eval_js`
- **Parameters:** `{ "expression": "document.title" }` (required)
- **Returns:** The evaluation result as a JSON value

## BrowserConfig

The `BrowserConfig` struct controls how the tools connect to Chrome:

```rust,ignore
use synaptic::browser::BrowserConfig;

// Use the default (localhost:9222)
let config = BrowserConfig::default();
assert_eq!(config.debug_url, "http://localhost:9222");

// Point to a custom debug URL
let config = BrowserConfig {
    debug_url: "http://192.168.1.50:9222".to_string(),
};
```

| Field       | Type     | Default                    | Description                          |
|-------------|----------|----------------------------|--------------------------------------|
| `debug_url` | `String` | `"http://localhost:9222"`  | Chrome DevTools Protocol debug URL   |

## Usage with Deep Agent

Inject browser tools into a Deep Agent via `DeepAgentOptions`:

```rust,ignore
use std::sync::Arc;
use synaptic::browser::{BrowserConfig, browser_tools};
use synaptic::deep::create_deep_agent;
use synaptic::openai::OpenAiChatModel;

let model = Arc::new(OpenAiChatModel::new("gpt-4o"));
let config = BrowserConfig::default();

// Get the browser tool set
let mut tools: Vec<Arc<dyn synaptic::core::Tool>> = browser_tools(&config);

// Add any additional tools
// tools.push(Arc::new(my_other_tool));

let agent = create_deep_agent(model, tools);
```

The agent can then autonomously decide when to navigate pages, read content via JavaScript evaluation, or capture screenshots during its reasoning loop.

## Usage with create_agent

You can also use browser tools with a standard ReAct agent:

```rust,ignore
use std::sync::Arc;
use synaptic::browser::{BrowserConfig, browser_tools};
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::openai::OpenAiChatModel;

let model = Arc::new(OpenAiChatModel::new("gpt-4o"));
let config = BrowserConfig::default();
let tools = browser_tools(&config);

let options = AgentOptions {
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

## Example: Scraping a Page Title

```rust,ignore
use synaptic::browser::{BrowserConfig, NavigateTool, EvalJsTool};
use synaptic::core::Tool;
use serde_json::json;

let config = BrowserConfig::default();

// Navigate to a page
let nav = NavigateTool::new(config.clone());
nav.call(json!({"url": "https://example.com"})).await?;

// Extract the page title
let eval = EvalJsTool::new(config);
let title = eval.call(json!({"expression": "document.title"})).await?;
println!("Page title: {}", title);
```

## MCP Alternative

For production browser automation, consider using an MCP browser server instead of direct CDP. MCP-based solutions such as the Playwright MCP server provide:

- Full CDP protocol coverage (clicks, form filling, waiting, etc.)
- Managed browser lifecycle (automatic launch and cleanup)
- Better error handling and session isolation
- Support for multiple concurrent pages

To use MCP browser tools with Synaptic, configure an MCP server in your agent config and the tools will be automatically discovered and registered. See the [MCP integration guide](../concepts/mcp.md) for details.

The `synaptic-browser` crate is best suited for lightweight automation tasks, prototyping, and environments where an external MCP server is not available.
