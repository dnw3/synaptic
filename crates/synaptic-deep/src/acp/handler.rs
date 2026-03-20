//! ACP method router — dispatches JSON-RPC requests to handler functions.

use serde_json::Value;

use super::types::*;

/// Routes ACP JSON-RPC requests to the appropriate handler.
///
/// This is a framework-level dispatcher. The actual agent factory and execution
/// are injected by the application (e.g., Synapse CLI) that embeds this handler.
pub struct AcpHandler {
    /// Application name reported in capabilities.
    pub name: String,
    /// Application version reported in capabilities.
    pub version: String,
    /// Tool descriptions for capabilities response.
    pub tools: Vec<ToolDescription>,
    /// Skill names for capabilities response.
    pub skills: Vec<String>,
}

impl AcpHandler {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: Vec::new(),
            skills: Vec::new(),
        }
    }

    /// Dispatch a JSON-RPC request and return the response.
    ///
    /// For `agent/run`, the caller should handle actual execution externally
    /// and pass results back. This handler provides capabilities and method routing.
    pub fn handle_capabilities(&self, id: Option<Value>) -> JsonRpcResponse {
        let caps = AgentCapabilities {
            name: self.name.clone(),
            version: self.version.clone(),
            tools: self.tools.clone(),
            skills: self.skills.clone(),
            streaming: true,
            approval_required: false,
        };

        JsonRpcResponse::success(id, serde_json::to_value(caps).unwrap_or(Value::Null))
    }

    /// Route a parsed JSON-RPC request. Returns None for methods that require
    /// external handling (agent/run, agent/cancel).
    pub fn route(&self, req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        match req.method.as_str() {
            "agent/capabilities" => Some(self.handle_capabilities(req.id.clone())),
            "agent/run" | "agent/status" | "agent/cancel" => {
                // These require external handling — return None to signal the caller
                None
            }
            _ => Some(JsonRpcResponse::error(
                req.id.clone(),
                METHOD_NOT_FOUND,
                format!("method '{}' not found", req.method),
            )),
        }
    }

    /// Parse a raw JSON string into a JsonRpcRequest.
    #[allow(clippy::result_large_err)]
    pub fn parse_request(raw: &str) -> Result<JsonRpcRequest, JsonRpcResponse> {
        serde_json::from_str::<JsonRpcRequest>(raw)
            .map_err(|e| JsonRpcResponse::error(None, PARSE_ERROR, format!("parse error: {}", e)))
    }
}
