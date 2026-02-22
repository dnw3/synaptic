use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Lightweight internal HTTP helper for the Feishu Spreadsheet API.
pub(crate) struct SpreadsheetApi {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl SpreadsheetApi {
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub(crate) fn check(body: &Value, ctx: &str) -> Result<(), SynapticError> {
        let code = body["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            Err(SynapticError::Tool(format!(
                "Lark Spreadsheet API error ({ctx}) code={code}: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }

    /// Write values to a range (overwrites existing data).
    pub async fn write_values(
        &self,
        token: &str,
        range: &str,
        values: Vec<Vec<Value>>,
    ) -> Result<(), SynapticError> {
        let auth_token = self.token_cache.get_token().await?;
        let url = format!("{}/sheets/v2/spreadsheets/{token}/values", self.base_url);
        let body = json!({ "valueRange": { "range": range, "values": values } });
        let resp = self
            .client
            .put(&url)
            .bearer_auth(&auth_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet write_values: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet write_values parse: {e}")))?;
        Self::check(&rb, "write_values")
    }

    /// Append rows after the last row in the range.
    pub async fn append_values(
        &self,
        token: &str,
        range: &str,
        values: Vec<Vec<Value>>,
    ) -> Result<(), SynapticError> {
        let auth_token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/sheets/v2/spreadsheets/{token}/values_append",
            self.base_url
        );
        let body = json!({
            "valueRange": { "range": range, "values": values },
            "insertDataOption": "INSERT_ROWS"
        });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&auth_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet append_values: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet append_values parse: {e}")))?;
        Self::check(&rb, "append_values")
    }

    /// Clear values in a range.
    pub async fn clear_values(&self, token: &str, range: &str) -> Result<(), SynapticError> {
        let auth_token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/sheets/v2/spreadsheets/{token}/values_batch_clear",
            self.base_url
        );
        let body = json!({ "ranges": [range] });
        let resp = self
            .client
            .put(&url)
            .bearer_auth(&auth_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet clear_values: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet clear_values parse: {e}")))?;
        Self::check(&rb, "clear_values")
    }

    /// Read values from a range.
    pub async fn read_values(
        &self,
        token: &str,
        range: &str,
    ) -> Result<Vec<Vec<Value>>, SynapticError> {
        let auth_token = self.token_cache.get_token().await?;
        let encoded_range = urlencoding::encode(range);
        let url = format!(
            "{}/sheets/v2/spreadsheets/{token}/values/{encoded_range}",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&auth_token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet read_values: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("spreadsheet read_values parse: {e}")))?;
        Self::check(&body, "read_values")?;
        Ok(body["data"]["valueRange"]["values"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|row| row.as_array().cloned().unwrap_or_default())
            .collect())
    }
}
