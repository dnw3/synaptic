use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use synapse_core::{SynapseError, Tool};
use synapse_tools::{SerialToolExecutor, ToolRegistry};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Echo input"
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, SynapseError> {
        Ok(json!({"echo": args}))
    }
}

#[tokio::test]
async fn executes_registered_tool() {
    let registry = ToolRegistry::new();
    registry.register(Arc::new(EchoTool)).expect("register");
    let executor = SerialToolExecutor::new(registry);

    let output = executor
        .execute("echo", json!({"msg":"hi"}))
        .await
        .expect("execute");

    assert_eq!(output, json!({"echo":{"msg":"hi"}}));
}

#[tokio::test]
async fn returns_error_for_unknown_tool() {
    let registry = ToolRegistry::new();
    let executor = SerialToolExecutor::new(registry);

    let err = executor
        .execute("missing", json!({}))
        .await
        .expect_err("should fail");

    assert!(matches!(err, SynapseError::ToolNotFound(name) if name == "missing"));
}
