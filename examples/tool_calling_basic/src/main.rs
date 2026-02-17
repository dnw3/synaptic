use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use synapse::core::{SynapseError, Tool};
use synapse::tools::{SerialToolExecutor, ToolRegistry};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Echo the given JSON payload"
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, SynapseError> {
        Ok(json!({ "echo": args }))
    }
}

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    let registry = ToolRegistry::new();
    registry.register(Arc::new(EchoTool))?;
    let executor = SerialToolExecutor::new(registry);

    let output = executor
        .execute("echo", json!({ "message": "hello from synapse" }))
        .await?;

    println!("{output}");
    Ok(())
}
