use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Lightweight internal HTTP helper for the Feishu Task API (v2).
pub(crate) struct TaskApi {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl TaskApi {
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
                "Lark Task API error ({ctx}) code={code}: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }

    /// List tasks (paginated).
    pub async fn list_tasks(
        &self,
        page_token: Option<&str>,
    ) -> Result<(Vec<Value>, Option<String>), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let mut url = format!("{}/task/v2/tasks?page_size=50", self.base_url);
        if let Some(pt) = page_token {
            url.push_str(&format!("&page_token={pt}"));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("task list_tasks: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("task list_tasks parse: {e}")))?;
        Self::check(&body, "list_tasks")?;
        let items = body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let next = body["data"]["page_token"].as_str().map(String::from);
        let has_more = body["data"]["has_more"].as_bool().unwrap_or(false);
        Ok((items, if has_more { next } else { None }))
    }

    /// Get a single task by GUID.
    pub async fn get_task(&self, task_guid: &str) -> Result<Value, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/task/v2/tasks/{task_guid}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("task get_task: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("task get_task parse: {e}")))?;
        Self::check(&body, "get_task")?;
        Ok(body["data"]["task"].clone())
    }

    /// Create a task and return its GUID.
    pub async fn create_task(
        &self,
        summary: &str,
        due_timestamp: Option<&str>,
        description: Option<&str>,
    ) -> Result<String, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/task/v2/tasks", self.base_url);
        let mut body = json!({ "summary": summary });
        if let Some(ts) = due_timestamp {
            body["due"] = json!({ "timestamp": ts });
        }
        if let Some(desc) = description {
            body["description"] = json!(desc);
        }
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("task create_task: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("task create_task parse: {e}")))?;
        Self::check(&rb, "create_task")?;
        Ok(rb["data"]["task"]["guid"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Update a task's fields.
    ///
    /// `fields` contains the task fields to update, and `update_fields` lists
    /// which keys are being updated.
    pub async fn update_task(
        &self,
        task_guid: &str,
        fields: Value,
        update_fields: Vec<String>,
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/task/v2/tasks/{task_guid}", self.base_url);
        let body = json!({ "task": fields, "update_fields": update_fields });
        let resp = self
            .client
            .patch(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("task update_task: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("task update_task parse: {e}")))?;
        Self::check(&rb, "update_task")
    }

    /// Mark a task as complete.
    pub async fn complete_task(&self, task_guid: &str) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/task/v2/tasks/{task_guid}/complete", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("task complete_task: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("task complete_task parse: {e}")))?;
        Self::check(&rb, "complete_task")
    }

    /// Delete a task.
    pub async fn delete_task(&self, task_guid: &str) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/task/v2/tasks/{task_guid}", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("task delete_task: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("task delete_task parse: {e}")))?;
        Self::check(&rb, "delete_task")
    }
}
