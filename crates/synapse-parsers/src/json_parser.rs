use async_trait::async_trait;
use serde_json::Value;
use synapse_core::{RunnableConfig, SynapseError};
use synapse_runnables::Runnable;

/// Parses a string as JSON, returning a `serde_json::Value`.
pub struct JsonOutputParser;

#[async_trait]
impl Runnable<String, Value> for JsonOutputParser {
    async fn invoke(&self, input: String, _config: &RunnableConfig) -> Result<Value, SynapseError> {
        serde_json::from_str(&input)
            .map_err(|e| SynapseError::Parsing(format!("invalid JSON: {e}")))
    }
}
