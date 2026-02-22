use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{api::contact::ContactApi, LarkConfig};

/// Look up Feishu/Lark users and departments as an Agent tool.
///
/// # Actions
///
/// | Action            | Description                                     |
/// |-------------------|-------------------------------------------------|
/// | `get_user`        | Get a user by ID                                |
/// | `batch_get_id`    | Resolve emails/mobiles to open_ids              |
/// | `list_departments`| List departments under a parent                 |
/// | `get_department`  | Get a department by ID                          |
pub struct LarkContactTool {
    api: ContactApi,
}

impl LarkContactTool {
    /// Create a new contact tool.
    pub fn new(config: LarkConfig) -> Self {
        Self {
            api: ContactApi::new(config),
        }
    }
}

#[async_trait]
impl Tool for LarkContactTool {
    fn name(&self) -> &'static str {
        "lark_contact"
    }

    fn description(&self) -> &'static str {
        "Look up Feishu/Lark users and departments. \
         Use action='get_user' to fetch a user by ID; \
         action='batch_get_id' to resolve emails or mobile numbers to open_ids; \
         action='list_departments' to list departments under a parent; \
         action='get_department' to fetch a department by ID."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: get_user | batch_get_id | list_departments | get_department",
                    "enum": ["get_user", "batch_get_id", "list_departments", "get_department"]
                },
                "user_id": {
                    "type": "string",
                    "description": "For 'get_user': the user ID to look up"
                },
                "user_id_type": {
                    "type": "string",
                    "description": "For 'get_user': ID type: open_id (default) | union_id | user_id",
                    "enum": ["open_id", "union_id", "user_id"]
                },
                "emails": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "For 'batch_get_id': list of email addresses to resolve"
                },
                "mobiles": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "For 'batch_get_id': list of mobile numbers to resolve"
                },
                "parent_department_id": {
                    "type": "string",
                    "description": "For 'list_departments': parent department ID (omit for root)"
                },
                "department_id": {
                    "type": "string",
                    "description": "For 'get_department': the department ID to look up"
                },
                "department_id_type": {
                    "type": "string",
                    "description": "For 'get_department': ID type: open_department_id (default) | department_id",
                    "enum": ["open_department_id", "department_id"]
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
            "get_user" => {
                let user_id = args["user_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'user_id'".to_string()))?;
                let id_type = args["user_id_type"].as_str().unwrap_or("open_id");
                let user = self.api.get_user(user_id, id_type).await?;
                Ok(json!({ "user": user }))
            }

            "batch_get_id" => {
                let emails: Vec<String> = args["emails"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let mobiles: Vec<String> = args["mobiles"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if emails.is_empty() && mobiles.is_empty() {
                    return Err(SynapticError::Tool(
                        "batch_get_id requires at least one of 'emails' or 'mobiles'".to_string(),
                    ));
                }
                let user_list = self.api.batch_get_id(&emails, &mobiles).await?;
                Ok(json!({ "user_list": user_list }))
            }

            "list_departments" => {
                let parent_id = args["parent_department_id"].as_str();
                let departments = self.api.list_departments(parent_id).await?;
                Ok(json!({ "departments": departments }))
            }

            "get_department" => {
                let dept_id = args["department_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'department_id'".to_string()))?;
                let id_type = args["department_id_type"]
                    .as_str()
                    .unwrap_or("open_department_id");
                let dept = self.api.get_department(dept_id, id_type).await?;
                Ok(json!({ "department": dept }))
            }

            other => Err(SynapticError::Tool(format!(
                "unknown action '{other}': expected get_user | batch_get_id | list_departments | get_department"
            ))),
        }
    }
}
