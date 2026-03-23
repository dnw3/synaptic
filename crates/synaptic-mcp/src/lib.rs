//! MCP (Model Context Protocol) adapters for connecting to external tool servers.
//!
//! This crate provides a [`MultiServerMcpClient`] that can connect to one or more
//! MCP-compatible servers over Stdio, SSE, or HTTP transports, discover their
//! advertised tools, and expose each tool as a [`synaptic_core::Tool`] implementor.

pub mod health;
pub mod oauth;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;

use synaptic_core::{SynapticError, Tool};

pub use health::{McpHealthHandle, McpHealthMonitor};
pub use oauth::{McpOAuthConfig, OAuthTokenManager};

// ---------------------------------------------------------------------------
// Connection types
// ---------------------------------------------------------------------------

/// Stdio transport connection config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioConnection {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// SSE (Server-Sent Events) transport connection config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseConnection {
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional OAuth 2.1 configuration for automatic token management.
    #[serde(default)]
    pub oauth: Option<McpOAuthConfig>,
}

/// HTTP (Streamable HTTP) transport connection config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConnection {
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional OAuth 2.1 configuration for automatic token management.
    #[serde(default)]
    pub oauth: Option<McpOAuthConfig>,
}

/// MCP server connection type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpConnection {
    Stdio(StdioConnection),
    Sse(SseConnection),
    Http(HttpConnection),
}

// ---------------------------------------------------------------------------
// StdioSession — persistent MCP subprocess connection
// ---------------------------------------------------------------------------

/// A persistent, initialized MCP stdio session.
///
/// Spawns the subprocess once during [`connect`], performs the MCP `initialize`
/// handshake, then reuses the same process for all subsequent `tools/list` and
/// `tools/call` requests. The child process is killed on drop.
struct StdioSession {
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    _child: tokio::process::Child,
    next_id: AtomicU64,
}

impl StdioSession {
    /// Spawn the subprocess and complete the MCP initialize handshake.
    async fn open(conn: &StdioConnection) -> Result<Self, SynapticError> {
        let mut child = tokio::process::Command::new(&conn.command)
            .args(&conn.args)
            .envs(&conn.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| SynapticError::Mcp(format!("Failed to spawn MCP process: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SynapticError::Mcp("Failed to open stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SynapticError::Mcp("Failed to open stdout".into()))?;
        let reader = BufReader::new(stdout);

        let mut session = Self {
            stdin,
            reader,
            _child: child,
            next_id: AtomicU64::new(1),
        };

        // MCP initialize handshake
        let _caps = session
            .rpc(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "synaptic",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
            .await?;

        // Send initialized notification (no response expected)
        session.notify("notifications/initialized").await?;

        Ok(session)
    }

    /// Send a JSON-RPC request and return the `result` field.
    async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, SynapticError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        });

        let msg = serde_json::to_string(&request).map_err(|e| SynapticError::Mcp(e.to_string()))?;
        self.stdin
            .write_all(msg.as_bytes())
            .await
            .map_err(|e| SynapticError::Mcp(e.to_string()))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| SynapticError::Mcp(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| SynapticError::Mcp(e.to_string()))?;

        // Read response (skip any notifications that arrive before the response)
        let mut line = String::new();
        loop {
            line.clear();
            self.reader
                .read_line(&mut line)
                .await
                .map_err(|e| SynapticError::Mcp(e.to_string()))?;

            if line.trim().is_empty() {
                return Err(SynapticError::Mcp(
                    "MCP process closed stdout unexpectedly".into(),
                ));
            }

            let body: Value = serde_json::from_str(&line)
                .map_err(|e| SynapticError::Mcp(format!("Failed to parse MCP response: {}", e)))?;

            // Skip notifications (no "id" field)
            if body.get("id").is_some() {
                if let Some(error) = body.get("error") {
                    let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                    let message = error
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown error");
                    return Err(SynapticError::Mcp(format!(
                        "MCP error {}: {}",
                        code, message
                    )));
                }
                return body
                    .get("result")
                    .cloned()
                    .ok_or_else(|| SynapticError::Mcp("No result in MCP response".into()));
            }
            // else: notification — loop and read next line
        }
    }

    /// Send a JSON-RPC notification (no id, no response expected).
    async fn notify(&mut self, method: &str) -> Result<(), SynapticError> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        let msg =
            serde_json::to_string(&notification).map_err(|e| SynapticError::Mcp(e.to_string()))?;
        self.stdin
            .write_all(msg.as_bytes())
            .await
            .map_err(|e| SynapticError::Mcp(e.to_string()))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| SynapticError::Mcp(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| SynapticError::Mcp(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// McpTool
// ---------------------------------------------------------------------------

/// A tool loaded from an MCP server.
struct McpTool {
    /// The tool name exposed to the model (may be prefixed with server name).
    tool_name: &'static str,
    /// The original tool name as registered on the MCP server.
    mcp_name: &'static str,
    tool_description: &'static str,
    tool_parameters: Value,
    /// Live stdio session (for Stdio transport).
    stdio_session: Option<Arc<tokio::sync::Mutex<StdioSession>>>,
    /// HTTP client + connection info (for Http/Sse transport).
    http: Option<HttpToolContext>,
}

struct HttpToolContext {
    client: reqwest::Client,
    url: String,
    headers: HashMap<String, String>,
    oauth_manager: Option<Arc<OAuthTokenManager>>,
}

/// Leak a `String` into a `&'static str`.
///
/// MCP tool definitions live for the entire program lifetime, so this
/// small, bounded leak is acceptable and avoids lifetime gymnastics.
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &'static str {
        self.tool_name
    }

    fn description(&self) -> &'static str {
        self.tool_description
    }

    fn parameters(&self) -> Option<Value> {
        Some(self.tool_parameters.clone())
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        if let Some(ref session) = self.stdio_session {
            // Stdio: use persistent session
            let mut sess = session.lock().await;
            sess.rpc(
                "tools/call",
                serde_json::json!({
                    "name": self.mcp_name,
                    "arguments": args,
                }),
            )
            .await
        } else if let Some(ref http) = self.http {
            // HTTP/SSE
            let headers = headers_with_oauth(&http.headers, http.oauth_manager.as_ref()).await?;
            call_http(&http.client, &http.url, &headers, self.mcp_name, &args).await
        } else {
            Err(SynapticError::Mcp("No connection available".into()))
        }
    }
}

/// Clone headers and inject OAuth Bearer token if configured.
async fn headers_with_oauth(
    base: &HashMap<String, String>,
    oauth: Option<&Arc<OAuthTokenManager>>,
) -> Result<HashMap<String, String>, SynapticError> {
    let mut headers = base.clone();
    if let Some(mgr) = oauth {
        let token = mgr.get_token().await?;
        headers.insert("Authorization".to_string(), format!("Bearer {}", token));
    }
    Ok(headers)
}

// ---------------------------------------------------------------------------
// HTTP transport helpers
// ---------------------------------------------------------------------------

/// Issue a JSON-RPC `tools/call` over HTTP(S) and return the `result` field.
async fn call_http(
    client: &reqwest::Client,
    url: &str,
    headers: &HashMap<String, String>,
    tool_name: &str,
    args: &Value,
) -> Result<Value, SynapticError> {
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": args,
        },
        "id": 1
    });

    let mut builder = client.post(url);
    for (key, value) in headers {
        builder = builder.header(key.as_str(), value.as_str());
    }
    builder = builder.header("Content-Type", "application/json");

    let resp = builder
        .json(&request_body)
        .send()
        .await
        .map_err(|e| SynapticError::Mcp(format!("HTTP request failed: {}", e)))?;

    let body: Value = resp
        .json()
        .await
        .map_err(|e| SynapticError::Mcp(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = body.get("error") {
        return Err(SynapticError::Mcp(format!("MCP error: {}", error)));
    }

    body.get("result")
        .cloned()
        .ok_or_else(|| SynapticError::Mcp("No result in MCP response".to_string()))
}

/// Issue a JSON-RPC `tools/list` over HTTP(S) and return the array of tool
/// definitions from `result.tools`.
async fn list_tools_http(
    client: &reqwest::Client,
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<Value, SynapticError> {
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "params": {},
        "id": 1
    });

    let mut builder = client.post(url);
    for (key, value) in headers {
        builder = builder.header(key.as_str(), value.as_str());
    }
    builder = builder.header("Content-Type", "application/json");

    let resp = builder
        .json(&request_body)
        .send()
        .await
        .map_err(|e| SynapticError::Mcp(e.to_string()))?;

    let body: Value = resp
        .json()
        .await
        .map_err(|e| SynapticError::Mcp(e.to_string()))?;

    Ok(body
        .get("result")
        .and_then(|r| r.get("tools"))
        .cloned()
        .unwrap_or(Value::Array(vec![])))
}

// ---------------------------------------------------------------------------
// Per-server discovery (free function for parallel spawning)
// ---------------------------------------------------------------------------

/// Discover tools from a single MCP server. For stdio servers, opens a persistent
/// session and stores it in `sessions` for reuse by tool calls.
async fn discover_server_tools(
    server_name: &str,
    connection: &McpConnection,
    client: &reqwest::Client,
    oauth_manager: Option<Arc<OAuthTokenManager>>,
    prefix_tool_names: bool,
    sessions: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<StdioSession>>>>>,
) -> Result<Vec<Arc<dyn Tool>>, SynapticError> {
    let (tools_list, stdio_session) = match connection {
        McpConnection::Http(conn) => {
            let list = list_tools_http(client, &conn.url, &conn.headers).await?;
            (list, None)
        }
        McpConnection::Sse(conn) => {
            let list = list_tools_http(client, &conn.url, &conn.headers).await?;
            (list, None)
        }
        McpConnection::Stdio(conn) => {
            let session = StdioSession::open(conn).await?;
            let session = Arc::new(tokio::sync::Mutex::new(session));

            let list = {
                let mut sess = session.lock().await;
                let result = sess.rpc("tools/list", serde_json::json!({})).await?;
                result.get("tools").cloned().unwrap_or(Value::Array(vec![]))
            };

            sessions
                .write()
                .await
                .insert(server_name.to_string(), session.clone());

            (list, Some(session))
        }
    };

    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

    if let Value::Array(tool_arr) = tools_list {
        for tool_def in tool_arr {
            let name = tool_def
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let description = tool_def
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let parameters = tool_def
                .get("inputSchema")
                .cloned()
                .unwrap_or(serde_json::json!({"type": "object"}));

            let mcp_name = leak_string(name.clone());
            let tool_name = if prefix_tool_names {
                leak_string(format!("{}_{}", server_name, name))
            } else {
                mcp_name
            };

            let http_ctx = match connection {
                McpConnection::Http(conn) => Some(HttpToolContext {
                    client: client.clone(),
                    url: conn.url.clone(),
                    headers: conn.headers.clone(),
                    oauth_manager: oauth_manager.clone(),
                }),
                McpConnection::Sse(conn) => Some(HttpToolContext {
                    client: client.clone(),
                    url: conn.url.clone(),
                    headers: conn.headers.clone(),
                    oauth_manager: oauth_manager.clone(),
                }),
                McpConnection::Stdio(_) => None,
            };

            tools.push(Arc::new(McpTool {
                tool_name,
                mcp_name,
                tool_description: leak_string(description),
                tool_parameters: parameters,
                stdio_session: stdio_session.clone(),
                http: http_ctx,
            }));
        }
    }

    Ok(tools)
}

// ---------------------------------------------------------------------------
// MultiServerMcpClient
// ---------------------------------------------------------------------------

/// Client that connects to one or more MCP servers and aggregates their tools.
pub struct MultiServerMcpClient {
    servers: HashMap<String, McpConnection>,
    prefix_tool_names: bool,
    tools: Arc<RwLock<Vec<Arc<dyn Tool>>>>,
    /// Cached OAuth token managers, keyed by server name.
    oauth_managers: Arc<RwLock<HashMap<String, Arc<OAuthTokenManager>>>>,
    /// Persistent stdio sessions, keyed by server name.
    stdio_sessions: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<StdioSession>>>>>,
}

impl MultiServerMcpClient {
    /// Create a new client with the given server map.
    pub fn new(servers: HashMap<String, McpConnection>) -> Self {
        Self {
            servers,
            prefix_tool_names: true,
            tools: Arc::new(RwLock::new(Vec::new())),
            oauth_managers: Arc::new(RwLock::new(HashMap::new())),
            stdio_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// When `true` (the default), discovered tool names are prefixed with the
    /// server name (e.g. `"myserver_tool_name"`).
    pub fn with_prefix(mut self, prefix: bool) -> Self {
        self.prefix_tool_names = prefix;
        self
    }

    /// Connect to all servers and discover available tools.
    ///
    /// For stdio servers, this spawns the subprocess and performs the MCP
    /// initialize handshake. The session is reused for all subsequent tool calls.
    pub async fn connect(&self) -> Result<(), SynapticError> {
        let client = reqwest::Client::new();
        let mut all_tools = Vec::new();

        // Build OAuth managers for connections that have oauth config.
        let mut managers = self.oauth_managers.write().await;
        for (server_name, connection) in &self.servers {
            let oauth_config = match connection {
                McpConnection::Http(conn) => conn.oauth.as_ref(),
                McpConnection::Sse(conn) => conn.oauth.as_ref(),
                McpConnection::Stdio(_) => None,
            };
            if let Some(config) = oauth_config {
                if !managers.contains_key(server_name) {
                    managers.insert(
                        server_name.clone(),
                        Arc::new(OAuthTokenManager::new(config.clone())),
                    );
                }
            }
        }
        drop(managers);

        // Connect to all servers in parallel
        let mut handles = Vec::new();
        for (server_name, connection) in &self.servers {
            let oauth_manager = self.oauth_managers.read().await.get(server_name).cloned();
            let name = server_name.clone();
            let conn = connection.clone();
            let cli = client.clone();
            let prefix = self.prefix_tool_names;
            let sessions = self.stdio_sessions.clone();
            handles.push(tokio::spawn(async move {
                discover_server_tools(&name, &conn, &cli, oauth_manager, prefix, sessions).await
            }));
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(tools)) => all_tools.extend(tools),
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "failed to connect to MCP server");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "MCP connection task panicked");
                }
            }
        }

        *self.tools.write().await = all_tools;
        Ok(())
    }

    /// Get all discovered tools.
    pub async fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.read().await.clone()
    }
}

// ---------------------------------------------------------------------------
// Convenience function
// ---------------------------------------------------------------------------

/// Convenience function to connect to all servers and return the discovered
/// tools in a single call.
pub async fn load_mcp_tools(
    client: &MultiServerMcpClient,
) -> Result<Vec<Arc<dyn Tool>>, SynapticError> {
    client.connect().await?;
    Ok(client.get_tools().await)
}
