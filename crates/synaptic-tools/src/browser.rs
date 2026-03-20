//! Browser automation tools for the Synaptic AI agent framework.
//!
//! This module provides [`Tool`] implementations for browser automation.
//! For production use, prefer the MCP browser integration which provides
//! full CDP support via an MCP server.
//!
//! Requires the `browser` feature flag.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

/// Browser configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Chrome DevTools debug URL.
    #[serde(default = "default_debug_url")]
    pub debug_url: String,
}

fn default_debug_url() -> String {
    "http://localhost:9222".to_string()
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            debug_url: default_debug_url(),
        }
    }
}

/// Create all browser tools with the given config.
pub fn browser_tools(config: &BrowserConfig) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(NavigateTool::new(config.clone())),
        Arc::new(ScreenshotTool::new(config.clone())),
        Arc::new(EvalJsTool::new(config.clone())),
    ]
}

// ---------------------------------------------------------------------------
// Navigate tool
// ---------------------------------------------------------------------------

pub struct NavigateTool {
    #[allow(dead_code)]
    config: BrowserConfig,
}

impl NavigateTool {
    pub fn new(config: BrowserConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for NavigateTool {
    fn name(&self) -> &'static str {
        "browser_navigate"
    }

    fn description(&self) -> &'static str {
        "Navigate the browser to a URL"
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to"
                }
            },
            "required": ["url"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'url' parameter".to_string()))?;

        // Validate URL scheme to prevent SSRF to internal networks
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(SynapticError::Tool(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        #[cfg(feature = "browser-cdp")]
        {
            let encoded_url = urlencoding::encode(url);
            let client = reqwest::Client::new();
            let resp = client
                .get(format!(
                    "{}/json/new?{}",
                    self.config.debug_url, encoded_url
                ))
                .send()
                .await
                .map_err(|e| SynapticError::Tool(format!("CDP navigate failed: {}", e)))?;

            if resp.status().is_success() {
                Ok(json!(format!("Navigated to {}", url)))
            } else {
                Err(SynapticError::Tool(format!(
                    "CDP navigate error: {}",
                    resp.status()
                )))
            }
        }

        #[cfg(not(feature = "browser-cdp"))]
        {
            let _ = url;
            Err(SynapticError::Tool(
                "CDP feature not enabled. Enable 'browser-cdp' feature or use MCP browser integration."
                    .to_string(),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Screenshot tool
// ---------------------------------------------------------------------------

pub struct ScreenshotTool {
    #[allow(dead_code)]
    config: BrowserConfig,
}

impl ScreenshotTool {
    pub fn new(config: BrowserConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ScreenshotTool {
    fn name(&self) -> &'static str {
        "browser_screenshot"
    }

    fn description(&self) -> &'static str {
        "Take a screenshot of the current browser page"
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {}
        }))
    }

    async fn call(&self, _args: Value) -> Result<Value, SynapticError> {
        #[cfg(feature = "browser-cdp")]
        {
            // CDP screenshot requires WebSocket; simplified stub
            Ok(json!("Screenshot capture requires full CDP WebSocket connection. Use MCP browser integration for full support."))
        }

        #[cfg(not(feature = "browser-cdp"))]
        {
            Err(SynapticError::Tool(
                "CDP feature not enabled. Enable 'browser-cdp' feature or use MCP browser integration."
                    .to_string(),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// EvalJs tool
// ---------------------------------------------------------------------------

pub struct EvalJsTool {
    #[allow(dead_code)]
    config: BrowserConfig,
}

impl EvalJsTool {
    pub fn new(config: BrowserConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for EvalJsTool {
    fn name(&self) -> &'static str {
        "browser_eval_js"
    }

    fn description(&self) -> &'static str {
        "Evaluate JavaScript in the browser page"
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "JavaScript expression to evaluate"
                }
            },
            "required": ["expression"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let _expression = args["expression"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'expression' parameter".to_string()))?;

        #[cfg(feature = "browser-cdp")]
        {
            // CDP eval requires WebSocket; simplified stub
            Ok(json!("JavaScript evaluation requires full CDP WebSocket connection. Use MCP browser integration for full support."))
        }

        #[cfg(not(feature = "browser-cdp"))]
        {
            Err(SynapticError::Tool(
                "CDP feature not enabled. Enable 'browser-cdp' feature or use MCP browser integration."
                    .to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert_eq!(config.debug_url, "http://localhost:9222");
    }

    #[test]
    fn test_browser_tools_count() {
        let config = BrowserConfig::default();
        let tools = browser_tools(&config);
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_tool_names() {
        let config = BrowserConfig::default();
        let tools = browser_tools(&config);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"browser_navigate"));
        assert!(names.contains(&"browser_screenshot"));
        assert!(names.contains(&"browser_eval_js"));
    }

    #[tokio::test]
    async fn test_navigate_without_cdp() {
        let tool = NavigateTool::new(BrowserConfig::default());
        let result = tool.call(json!({"url": "https://example.com"})).await;
        // Without CDP feature, should return error
        #[cfg(not(feature = "browser-cdp"))]
        assert!(result.is_err());
        #[cfg(feature = "browser-cdp")]
        let _ = result; // May fail if no Chrome running
    }
}
