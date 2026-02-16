use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardrailError {
    #[error("invalid json: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("expected json object")]
    ExpectedObject,
}

pub struct JsonObjectGuard;

impl JsonObjectGuard {
    pub fn validate(input: &str) -> Result<Value, GuardrailError> {
        let value: Value = serde_json::from_str(input)?;
        if value.is_object() {
            Ok(value)
        } else {
            Err(GuardrailError::ExpectedObject)
        }
    }
}
