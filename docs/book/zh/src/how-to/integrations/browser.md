# 浏览器自动化

基于 CDP（Chrome DevTools Protocol）的浏览器工具，供 Synaptic agent 使用。

## 简介

`synaptic-browser` crate 提供了一组实现了 `Tool` trait 的浏览器自动化工具。这些工具通过 CDP 协议与 Chrome 浏览器通信，允许 Agent 导航网页、截取屏幕截图和执行 JavaScript。

对于生产环境中需要完整 CDP 功能的场景，推荐使用 MCP 浏览器集成（参见本页最后一节）。

## 安装

在 `Cargo.toml` 中添加 `browser` feature：

```toml
[dependencies]
synaptic = { version = "0.3", features = ["browser"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

**前置条件：** 需要以远程调试模式启动 Chrome 浏览器：

```bash
# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
    --remote-debugging-port=9222

# Linux
google-chrome --remote-debugging-port=9222

# Windows
chrome.exe --remote-debugging-port=9222
```

启动后，Chrome 会在 `http://localhost:9222` 上监听 CDP 连接。

## 可用工具

`synaptic-browser` 提供三个内置工具：

| 工具名称 | 描述 | 参数 |
|----------|------|------|
| `browser_navigate` | 导航浏览器到指定 URL | `url: String` |
| `browser_screenshot` | 截取当前页面的屏幕截图 | 无 |
| `browser_eval_js` | 在浏览器页面中执行 JavaScript | `expression: String` |

### NavigateTool

导航到指定的 URL：

```rust,ignore
use synaptic::browser::{BrowserConfig, NavigateTool};
use synaptic::core::Tool;

let config = BrowserConfig::default();
let tool = NavigateTool::new(config);

let result = tool.call(serde_json::json!({
    "url": "https://example.com"
})).await?;
```

### ScreenshotTool

截取当前页面的屏幕截图，返回 base64 编码的 PNG 数据：

```rust,ignore
use synaptic::browser::{BrowserConfig, ScreenshotTool};
use synaptic::core::Tool;

let config = BrowserConfig::default();
let tool = ScreenshotTool::new(config);

let result = tool.call(serde_json::json!({})).await?;
```

### EvalJsTool

在浏览器页面中执行 JavaScript 表达式：

```rust,ignore
use synaptic::browser::{BrowserConfig, EvalJsTool};
use synaptic::core::Tool;

let config = BrowserConfig::default();
let tool = EvalJsTool::new(config);

let result = tool.call(serde_json::json!({
    "expression": "document.title"
})).await?;
```

## BrowserConfig

`BrowserConfig` 用于配置 CDP 连接地址：

```rust,ignore
use synaptic::browser::BrowserConfig;

// 使用默认配置（localhost:9222）
let config = BrowserConfig::default();

// 自定义调试地址
let config = BrowserConfig {
    debug_url: "http://192.168.1.100:9222".to_string(),
};
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `debug_url` | `String` | `"http://localhost:9222"` | Chrome DevTools 调试 URL |

## 与 Deep Agent 集成

使用 `browser_tools()` 函数批量创建所有浏览器工具，然后注入到 Deep Agent 中：

```rust,ignore
use std::sync::Arc;
use synaptic::browser::{BrowserConfig, browser_tools};
use synaptic::deep::{DeepAgentOptions, create_deep_agent};

let config = BrowserConfig::default();
let tools = browser_tools(&config);

// 将浏览器工具与其他工具合并
let mut all_tools: Vec<Arc<dyn synaptic::core::Tool>> = vec![];
all_tools.extend(tools);

// 传入 Agent 配置
let options = DeepAgentOptions {
    tools: all_tools,
    ..Default::default()
};
```

`browser_tools(&config)` 返回一个 `Vec<Arc<dyn Tool>>`，包含所有三个浏览器工具（NavigateTool、ScreenshotTool、EvalJsTool）。

## 与 ReAct Agent 集成

也可以将浏览器工具注册到 ReAct Agent 中：

```rust,ignore
use std::sync::Arc;
use synaptic::browser::{BrowserConfig, browser_tools};
use synaptic::graph::create_react_agent;
use synaptic::openai::OpenAiChatModel;

let config = BrowserConfig::default();
let tools = browser_tools(&config);

let model = Arc::new(OpenAiChatModel::new("gpt-4o"));
let graph = create_react_agent(model, tools).compile()?;
```

## MCP 替代方案

对于生产环境，推荐使用 MCP (Model Context Protocol) 浏览器 server。MCP 方案提供以下优势：

- 完整的 CDP WebSocket 支持，包括截图、DOM 操作等高级功能。
- 独立进程运行，与 Agent 解耦。
- 支持多种传输方式（Stdio、SSE、HTTP）。

```rust,ignore
use synaptic::mcp::MultiServerMcpClient;

let client = MultiServerMcpClient::from_config(vec![
    ("browser".to_string(), synaptic::mcp::McpServerConfig {
        transport: synaptic::mcp::McpTransport::Sse {
            url: "http://localhost:3000/sse".to_string(),
        },
        ..Default::default()
    }),
]).await?;

let tools = client.list_tools().await?;
```

内置的 `synaptic-browser` 工具适用于原型开发和简单场景；对于需要完整浏览器控制的生产用例，MCP 浏览器 server 是更成熟的选择。
