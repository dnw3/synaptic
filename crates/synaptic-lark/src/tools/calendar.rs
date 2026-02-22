use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{api::calendar::CalendarApi, LarkConfig};

/// Manage Feishu/Lark Calendar events as an Agent tool.
///
/// # Actions
///
/// | Action            | Description                                  |
/// |-------------------|----------------------------------------------|
/// | `list_calendars`  | List accessible calendars                    |
/// | `list_events`     | List events in a calendar                    |
/// | `get_event`       | Get details of a specific event              |
/// | `create_event`    | Create a new event                           |
/// | `update_event`    | Update an existing event                     |
/// | `delete_event`    | Delete an event                              |
pub struct LarkCalendarTool {
    api: CalendarApi,
}

impl LarkCalendarTool {
    /// Create a new calendar tool.
    pub fn new(config: LarkConfig) -> Self {
        Self {
            api: CalendarApi::new(config),
        }
    }
}

#[async_trait]
impl Tool for LarkCalendarTool {
    fn name(&self) -> &'static str {
        "lark_calendar"
    }

    fn description(&self) -> &'static str {
        "Manage Feishu/Lark Calendar events. \
         Use action='list_calendars' to list calendars; \
         action='list_events' to list events (with optional time range); \
         action='get_event' to get event details; \
         action='create_event' to create an event; \
         action='update_event' to update an event; \
         action='delete_event' to delete an event. \
         Times are Unix timestamp strings (seconds)."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: list_calendars | list_events | get_event | create_event | update_event | delete_event",
                    "enum": ["list_calendars", "list_events", "get_event", "create_event", "update_event", "delete_event"]
                },
                "calendar_id": {
                    "type": "string",
                    "description": "Calendar ID — required for list_events, get_event, create_event, update_event, delete_event"
                },
                "event_id": {
                    "type": "string",
                    "description": "Event ID — required for get_event, update_event, delete_event"
                },
                "summary": {
                    "type": "string",
                    "description": "For 'create_event': event title (required); for 'update_event': new title"
                },
                "description": {
                    "type": "string",
                    "description": "For 'create_event'/'update_event': event description"
                },
                "start_time": {
                    "type": "string",
                    "description": "Unix timestamp string (seconds) — required for create_event; optional filter for list_events"
                },
                "end_time": {
                    "type": "string",
                    "description": "Unix timestamp string (seconds) — required for create_event; optional filter for list_events"
                }
            },
            "required": ["action"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'action'".to_string()))?;

        match action {
            "list_calendars" => {
                let calendars = self.api.list_calendars().await?;
                Ok(json!({ "calendars": calendars }))
            }

            "list_events" => {
                let calendar_id = args["calendar_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'calendar_id'".to_string()))?;
                let start_time = args["start_time"].as_str();
                let end_time = args["end_time"].as_str();
                let events = self
                    .api
                    .list_events(calendar_id, start_time, end_time)
                    .await?;
                Ok(json!({ "events": events }))
            }

            "get_event" => {
                let calendar_id = args["calendar_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'calendar_id'".to_string()))?;
                let event_id = args["event_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'event_id'".to_string()))?;
                let event = self.api.get_event(calendar_id, event_id).await?;
                Ok(json!({ "event": event }))
            }

            "create_event" => {
                let calendar_id = args["calendar_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'calendar_id'".to_string()))?;
                let summary = args["summary"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'summary'".to_string()))?;
                let start_time = args["start_time"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'start_time'".to_string()))?;
                let end_time = args["end_time"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'end_time'".to_string()))?;
                let description = args["description"].as_str();
                let event_id = self
                    .api
                    .create_event(calendar_id, summary, start_time, end_time, description)
                    .await?;
                Ok(json!({ "event_id": event_id }))
            }

            "update_event" => {
                let calendar_id = args["calendar_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'calendar_id'".to_string()))?;
                let event_id = args["event_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'event_id'".to_string()))?;
                let mut fields = json!({});
                if let Some(s) = args["summary"].as_str() {
                    fields["summary"] = json!(s);
                }
                if let Some(d) = args["description"].as_str() {
                    fields["description"] = json!(d);
                }
                if let Some(st) = args["start_time"].as_str() {
                    fields["start_time"] = json!({ "timestamp": st });
                }
                if let Some(et) = args["end_time"].as_str() {
                    fields["end_time"] = json!({ "timestamp": et });
                }
                self.api.update_event(calendar_id, event_id, fields).await?;
                Ok(json!({ "event_id": event_id, "status": "updated" }))
            }

            "delete_event" => {
                let calendar_id = args["calendar_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'calendar_id'".to_string()))?;
                let event_id = args["event_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'event_id'".to_string()))?;
                self.api.delete_event(calendar_id, event_id).await?;
                Ok(json!({ "status": "deleted" }))
            }

            other => Err(SynapticError::Tool(format!(
                "unknown action '{other}': expected list_calendars | list_events | get_event | create_event | update_event | delete_event"
            ))),
        }
    }
}
