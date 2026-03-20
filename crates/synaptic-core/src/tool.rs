use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::SynapticError;
use crate::store::Store;
use crate::types::{RunnableConfig, StreamWriter};

// ---------------------------------------------------------------------------
// Tool-related types
// ---------------------------------------------------------------------------

/// Represents a tool invocation requested by an AI model, with an ID, function name, and JSON arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// A tool call that failed to parse correctly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidToolCall {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    pub error: String,
}

/// A partial tool call chunk received during streaming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallChunk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

/// Schema definition for a tool, including its name, description, and JSON Schema for parameters.
///
/// The `input_schema` field follows the MCP / Anthropic convention. When serialized it also
/// accepts the aliases `"inputSchema"` and `"parameters"` for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(alias = "inputSchema", alias = "parameters")]
    pub input_schema: Value,
    /// Provider-specific parameters (e.g., Anthropic's `cache_control`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, Value>>,
}

/// Controls how the model selects tools: Auto, Required, None, or a Specific named tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    Auto,
    Required,
    None,
    Specific(String),
}

// ---------------------------------------------------------------------------
// Tool trait
// ---------------------------------------------------------------------------

/// Defines an executable tool that can be called by an AI model. Each tool has a name, description, JSON schema for parameters, and an async `call()` method.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn parameters(&self) -> Option<Value> {
        None
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError>;

    fn as_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self
                .parameters()
                .unwrap_or(serde_json::json!({"type": "object", "properties": {}})),
            extras: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ToolRuntime
// ---------------------------------------------------------------------------

/// Tool execution runtime context.
#[derive(Clone)]
pub struct ToolRuntime {
    pub store: Option<Arc<dyn Store>>,
    pub stream_writer: Option<StreamWriter>,
    pub state: Option<Value>,
    pub tool_call_id: String,
    pub config: Option<RunnableConfig>,
}

// ---------------------------------------------------------------------------
// RuntimeAwareTool
// ---------------------------------------------------------------------------

/// Context-aware tool that receives runtime information.
///
/// This extends the basic `Tool` trait with runtime context
/// (current state, store, stream writer, tool call ID). Implement this
/// for tools that need to read or modify graph state.
#[async_trait]
pub trait RuntimeAwareTool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn parameters(&self) -> Option<Value> {
        None
    }

    async fn call_with_runtime(
        &self,
        args: Value,
        runtime: ToolRuntime,
    ) -> Result<Value, SynapticError>;

    fn as_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self
                .parameters()
                .unwrap_or(serde_json::json!({"type": "object", "properties": {}})),
            extras: None,
        }
    }
}

/// Adapter that wraps a `RuntimeAwareTool` into a standard `Tool`.
///
/// When used outside a graph context, the tool receives a default
/// (empty) `ToolRuntime`.
pub struct RuntimeAwareToolAdapter {
    inner: Arc<dyn RuntimeAwareTool>,
    runtime: Arc<tokio::sync::RwLock<Option<ToolRuntime>>>,
}

impl RuntimeAwareToolAdapter {
    pub fn new(tool: Arc<dyn RuntimeAwareTool>) -> Self {
        Self {
            inner: tool,
            runtime: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    pub async fn set_runtime(&self, runtime: ToolRuntime) {
        *self.runtime.write().await = Some(runtime);
    }
}

#[async_trait]
impl Tool for RuntimeAwareToolAdapter {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn description(&self) -> &'static str {
        self.inner.description()
    }

    fn parameters(&self) -> Option<Value> {
        self.inner.parameters()
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let runtime = self.runtime.read().await.clone().unwrap_or(ToolRuntime {
            store: None,
            stream_writer: None,
            state: None,
            tool_call_id: String::new(),
            config: None,
        });
        self.inner.call_with_runtime(args, runtime).await
    }
}
