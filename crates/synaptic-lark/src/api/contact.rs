use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Lightweight internal HTTP helper for the Feishu Contact API.
pub(crate) struct ContactApi {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl ContactApi {
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
                "Lark Contact API error ({ctx}) code={code}: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }

    /// Get a single user by ID.
    pub async fn get_user(&self, user_id: &str, id_type: &str) -> Result<Value, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/contact/v3/users/{user_id}?user_id_type={id_type}",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact get_user: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact get_user parse: {e}")))?;
        Self::check(&body, "get_user")?;
        Ok(body["data"]["user"].clone())
    }

    /// Batch resolve emails/mobiles to open_ids.
    pub async fn batch_get_id(
        &self,
        emails: &[String],
        mobiles: &[String],
    ) -> Result<Value, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/contact/v3/users/batch_get_id", self.base_url);
        let mut body = json!({});
        if !emails.is_empty() {
            body["emails"] = json!(emails);
        }
        if !mobiles.is_empty() {
            body["mobiles"] = json!(mobiles);
        }
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact batch_get_id: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact batch_get_id parse: {e}")))?;
        Self::check(&rb, "batch_get_id")?;
        Ok(rb["data"]["user_list"].clone())
    }

    /// List departments under a parent department.
    pub async fn list_departments(
        &self,
        parent_dept_id: Option<&str>,
    ) -> Result<Vec<Value>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let mut url = format!("{}/contact/v3/departments?fetch_child=true", self.base_url);
        if let Some(id) = parent_dept_id {
            url.push_str(&format!("&parent_department_id={id}"));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact list_departments: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact list_departments parse: {e}")))?;
        Self::check(&body, "list_departments")?;
        Ok(body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Get a single department by ID.
    pub async fn get_department(
        &self,
        dept_id: &str,
        id_type: &str,
    ) -> Result<Value, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/contact/v3/departments/{dept_id}?department_id_type={id_type}",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact get_department: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("contact get_department parse: {e}")))?;
        Self::check(&body, "get_department")?;
        Ok(body["data"]["department"].clone())
    }
}
