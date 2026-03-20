use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

/// Configuration for an MCP server connection.
#[derive(Clone, Deserialize)]
pub struct McpServerConfig {
    /// Server name identifier.
    pub name: String,
    /// Transport type: "stdio", "sse", or "http".
    pub transport: String,
    /// Command to launch (for stdio transport).
    pub command: Option<String>,
    /// Command arguments (for stdio transport).
    pub args: Option<Vec<String>>,
    /// URL endpoint (for sse/http transport).
    pub url: Option<String>,
    /// Additional headers (for sse/http transport).
    pub headers: Option<HashMap<String, String>>,
    /// Environment variables to set when launching a stdio server.
    pub env: Option<HashMap<String, String>>,
}

// Custom Debug impl to redact sensitive fields (headers, env may contain secrets).
impl fmt::Debug for McpServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpServerConfig")
            .field("name", &self.name)
            .field("transport", &self.transport)
            .field("command", &self.command)
            .field("args", &self.args)
            .field("url", &self.url)
            .field(
                "headers",
                &self
                    .headers
                    .as_ref()
                    .map(|h| format!("<{} headers>", h.len())),
            )
            .field(
                "env",
                &self.env.as_ref().map(|e| format!("<{} vars>", e.len())),
            )
            .finish()
    }
}
