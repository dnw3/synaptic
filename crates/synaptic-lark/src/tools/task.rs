use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{api::task::TaskApi, LarkConfig};

/// Create and manage Feishu/Lark Tasks as an Agent tool.
///
/// # Actions
///
/// | Action     | Description                                    |
/// |------------|------------------------------------------------|
/// | `list`     | List all tasks (paginated)                     |
/// | `get`      | Get details of a specific task                 |
/// | `create`   | Create a new task                              |
/// | `update`   | Update an existing task                        |
/// | `complete` | Mark a task as complete                        |
/// | `delete`   | Delete a task                                  |
pub struct LarkTaskTool {
    api: TaskApi,
}

impl LarkTaskTool {
    /// Create a new task tool.
    pub fn new(config: LarkConfig) -> Self {
        Self {
            api: TaskApi::new(config),
        }
    }
}

#[async_trait]
impl Tool for LarkTaskTool {
    fn name(&self) -> &'static str {
        "lark_task"
    }

    fn description(&self) -> &'static str {
        "Create and manage Feishu/Lark Tasks. \
         Use action='list' to list tasks; \
         action='get' to get a task by GUID; \
         action='create' to create a new task; \
         action='update' to update task fields; \
         action='complete' to mark a task as complete; \
         action='delete' to delete a task."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: list | get | create | update | complete | delete",
                    "enum": ["list", "get", "create", "update", "complete", "delete"]
                },
                "task_guid": {
                    "type": "string",
                    "description": "Task GUID — required for get, update, complete, delete"
                },
                "summary": {
                    "type": "string",
                    "description": "For 'create': task title (required); for 'update': new title"
                },
                "description": {
                    "type": "string",
                    "description": "For 'create'/'update': task description"
                },
                "due_timestamp": {
                    "type": "string",
                    "description": "For 'create'/'update': due date as Unix timestamp string (seconds)"
                },
                "page_token": {
                    "type": "string",
                    "description": "For 'list': pagination token from previous response"
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
            "list" => {
                let page_token = args["page_token"].as_str();
                let (tasks, next_token) = self.api.list_tasks(page_token).await?;
                let mut result = json!({ "tasks": tasks });
                if let Some(t) = next_token {
                    result["next_page_token"] = json!(t);
                }
                Ok(result)
            }

            "get" => {
                let task_guid = args["task_guid"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'task_guid'".to_string()))?;
                let task = self.api.get_task(task_guid).await?;
                Ok(json!({ "task": task }))
            }

            "create" => {
                let summary = args["summary"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'summary'".to_string()))?;
                let due_timestamp = args["due_timestamp"].as_str();
                let description = args["description"].as_str();
                let task_guid = self
                    .api
                    .create_task(summary, due_timestamp, description)
                    .await?;
                Ok(json!({ "task_guid": task_guid }))
            }

            "update" => {
                let task_guid = args["task_guid"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'task_guid'".to_string()))?;
                let mut fields = json!({});
                let mut update_fields: Vec<String> = Vec::new();
                if let Some(s) = args["summary"].as_str() {
                    fields["summary"] = json!(s);
                    update_fields.push("summary".to_string());
                }
                if let Some(d) = args["description"].as_str() {
                    fields["description"] = json!(d);
                    update_fields.push("description".to_string());
                }
                if let Some(ts) = args["due_timestamp"].as_str() {
                    fields["due"] = json!({ "timestamp": ts });
                    update_fields.push("due".to_string());
                }
                self.api
                    .update_task(task_guid, fields, update_fields)
                    .await?;
                Ok(json!({ "task_guid": task_guid, "status": "updated" }))
            }

            "complete" => {
                let task_guid = args["task_guid"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'task_guid'".to_string()))?;
                self.api.complete_task(task_guid).await?;
                Ok(json!({ "task_guid": task_guid, "status": "completed" }))
            }

            "delete" => {
                let task_guid = args["task_guid"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'task_guid'".to_string()))?;
                self.api.delete_task(task_guid).await?;
                Ok(json!({ "task_guid": task_guid, "status": "deleted" }))
            }

            other => Err(SynapticError::Tool(format!(
                "unknown action '{other}': expected list | get | create | update | complete | delete"
            ))),
        }
    }
}
