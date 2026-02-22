use serde_json::{json, Value};
use synaptic_core::SynapticError;

use crate::{auth::TokenCache, LarkConfig};

/// Lightweight internal HTTP helper for the Feishu Calendar API.
pub(crate) struct CalendarApi {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
}

impl CalendarApi {
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
                "Lark Calendar API error ({ctx}) code={code}: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )))
        } else {
            Ok(())
        }
    }

    /// List all calendars accessible by the bot.
    pub async fn list_calendars(&self) -> Result<Vec<Value>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!("{}/calendar/v4/calendars", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar list_calendars: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar list_calendars parse: {e}")))?;
        Self::check(&body, "list_calendars")?;
        Ok(body["data"]["calendar_list"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// List events in a calendar, optionally filtered by time range.
    ///
    /// `start_time` and `end_time` are Unix timestamp strings (seconds).
    pub async fn list_events(
        &self,
        calendar_id: &str,
        start_time: Option<&str>,
        end_time: Option<&str>,
    ) -> Result<Vec<Value>, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let mut url = format!(
            "{}/calendar/v4/calendars/{calendar_id}/events",
            self.base_url
        );
        let mut sep = '?';
        if let Some(st) = start_time {
            url.push_str(&format!("{sep}start_time={st}"));
            sep = '&';
        }
        if let Some(et) = end_time {
            url.push_str(&format!("{sep}end_time={et}"));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar list_events: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar list_events parse: {e}")))?;
        Self::check(&body, "list_events")?;
        Ok(body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Get a single event.
    pub async fn get_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<Value, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/calendar/v4/calendars/{calendar_id}/events/{event_id}",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar get_event: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar get_event parse: {e}")))?;
        Self::check(&body, "get_event")?;
        Ok(body["data"]["event"].clone())
    }

    /// Create a new event and return its event_id.
    pub async fn create_event(
        &self,
        calendar_id: &str,
        summary: &str,
        start_ts: &str,
        end_ts: &str,
        description: Option<&str>,
    ) -> Result<String, SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/calendar/v4/calendars/{calendar_id}/events",
            self.base_url
        );
        let mut body = json!({
            "summary": summary,
            "start_time": { "timestamp": start_ts },
            "end_time": { "timestamp": end_ts }
        });
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
            .map_err(|e| SynapticError::Tool(format!("calendar create_event: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar create_event parse: {e}")))?;
        Self::check(&rb, "create_event")?;
        Ok(rb["data"]["event"]["event_id"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Update fields on an existing event.
    pub async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        fields: Value,
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/calendar/v4/calendars/{calendar_id}/events/{event_id}",
            self.base_url
        );
        let resp = self
            .client
            .patch(&url)
            .bearer_auth(&token)
            .json(&fields)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar update_event: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar update_event parse: {e}")))?;
        Self::check(&rb, "update_event")
    }

    /// Delete an event.
    pub async fn delete_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<(), SynapticError> {
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/calendar/v4/calendars/{calendar_id}/events/{event_id}",
            self.base_url
        );
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar delete_event: {e}")))?;
        let rb: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("calendar delete_event parse: {e}")))?;
        Self::check(&rb, "delete_event")
    }
}
