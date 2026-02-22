use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{ChatResponse, LlmCache, SynapticError};

use crate::{auth::TokenCache, LarkConfig};

/// A team-shared LLM response cache stored in a Feishu Bitable table.
///
/// Each row represents one cached response, keyed by `cache_key`. Hit counts
/// are tracked in a `hit_count` field and are visible directly in the Feishu
/// spreadsheet, making cache utilisation observable without additional tooling.
///
/// # Bitable table schema
///
/// | Field name      | Type   | Notes                         |
/// |-----------------|--------|-------------------------------|
/// | `cache_key`     | Text   | Unique cache key              |
/// | `response_json` | Text   | Serialised `ChatResponse`     |
/// | `hit_count`     | Text   | Number of cache hits (string) |
/// | `created_at`    | Text   | Unix timestamp (seconds)      |
pub struct LarkBitableLlmCache {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
    app_token: String,
    table_id: String,
}

impl LarkBitableLlmCache {
    /// Create a new cache backed by the given Bitable table.
    pub fn new(
        config: LarkConfig,
        app_token: impl Into<String>,
        table_id: impl Into<String>,
    ) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
            app_token: app_token.into(),
            table_id: table_id.into(),
        }
    }

    /// Return the Bitable application token.
    pub fn app_token(&self) -> &str {
        &self.app_token
    }

    /// Return the Bitable table ID.
    pub fn table_id(&self) -> &str {
        &self.table_id
    }

    fn search_url(&self) -> String {
        format!(
            "{}/bitable/v1/apps/{}/tables/{}/records/search",
            self.base_url, self.app_token, self.table_id
        )
    }

    fn records_url(&self) -> String {
        format!(
            "{}/bitable/v1/apps/{}/tables/{}/records",
            self.base_url, self.app_token, self.table_id
        )
    }

    fn check(body: &Value, ctx: &str) -> Result<(), SynapticError> {
        if body["code"].as_i64().unwrap_or(-1) != 0 {
            Err(SynapticError::Cache(format!(
                "LlmCache ({ctx}): {}",
                body["msg"].as_str().unwrap_or("?")
            )))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl LlmCache for LarkBitableLlmCache {
    async fn get(&self, key: &str) -> Result<Option<ChatResponse>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let body = json!({
            "page_size": 1,
            "filter": {
                "conjunction": "and",
                "conditions": [{
                    "field_name": "cache_key",
                    "operator": "is",
                    "value": [key]
                }]
            }
        });
        let resp = self
            .client
            .post(&self.search_url())
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache get: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache get parse: {e}")))?;
        Self::check(&rb, "get")?;

        let item = rb["data"]["items"]
            .as_array()
            .and_then(|a| a.first())
            .cloned();

        match item {
            None => Ok(None),
            Some(rec) => {
                let json_str = rec["fields"]["response_json"].as_str().unwrap_or("{}");
                let response: ChatResponse = serde_json::from_str(json_str)
                    .map_err(|e| SynapticError::Cache(format!("deserialize cache: {e}")))?;

                // Increment hit_count — fire-and-forget, ignore errors so a
                // counter update failure never breaks the caller.
                let record_id = rec["record_id"].as_str().unwrap_or("").to_string();
                let hit = rec["fields"]["hit_count"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0)
                    + 1;
                let update_body = json!({ "fields": { "hit_count": hit.to_string() } });
                let _ = self
                    .client
                    .put(format!("{}/{}", self.records_url(), record_id))
                    .bearer_auth(&token)
                    .json(&update_body)
                    .send()
                    .await;

                Ok(Some(response))
            }
        }
    }

    async fn put(&self, key: &str, response: &ChatResponse) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let json_str = serde_json::to_string(response)
            .map_err(|e| SynapticError::Cache(format!("serialize cache: {e}")))?;
        let body = json!({
            "records": [{
                "fields": {
                    "cache_key": key,
                    "response_json": json_str,
                    "hit_count": "0",
                    "created_at": now_ts(),
                }
            }]
        });
        let resp = self
            .client
            .post(format!("{}/batch_create", self.records_url()))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache put: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache put parse: {e}")))?;
        Self::check(&rb, "put")
    }

    async fn clear(&self) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        // Fetch all record IDs (up to 500 per page)
        let body = json!({ "page_size": 500 });
        let resp = self
            .client
            .post(&self.search_url())
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache clear search: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache clear parse: {e}")))?;
        Self::check(&rb, "clear/search")?;

        let ids: Vec<String> = rb["data"]["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|r| r["record_id"].as_str().map(String::from))
            .collect();

        if ids.is_empty() {
            return Ok(());
        }

        let del = json!({ "records": ids });
        let resp = self
            .client
            .delete(format!("{}/batch_delete", self.records_url()))
            .bearer_auth(&token)
            .json(&del)
            .send()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache clear delete: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Cache(format!("cache clear delete parse: {e}")))?;
        Self::check(&rb, "clear/delete")
    }
}

fn now_ts() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
