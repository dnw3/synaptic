use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{api::chat::ChatApi, LarkConfig};

/// Manage Feishu/Lark group chats as an Agent tool.
///
/// # Actions
///
/// | Action           | Description                                   |
/// |------------------|-----------------------------------------------|
/// | `list`           | List all chats the bot belongs to             |
/// | `get`            | Get details of a specific chat                |
/// | `create`         | Create a new group chat                       |
/// | `update`         | Update chat name or description               |
/// | `list_members`   | List members of a chat                        |
/// | `add_members`    | Add members to a chat                         |
/// | `remove_members` | Remove members from a chat                    |
pub struct LarkChatTool {
    api: ChatApi,
}

impl LarkChatTool {
    /// Create a new chat tool.
    pub fn new(config: LarkConfig) -> Self {
        Self {
            api: ChatApi::new(config),
        }
    }
}

#[async_trait]
impl Tool for LarkChatTool {
    fn name(&self) -> &'static str {
        "lark_chat"
    }

    fn description(&self) -> &'static str {
        "Manage Feishu/Lark group chats. \
         Use action='list' to list all chats; \
         action='get' to get a specific chat; \
         action='create' to create a new group chat; \
         action='update' to update chat settings; \
         action='list_members' to list members; \
         action='add_members' or action='remove_members' to manage membership."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: list | get | create | update | list_members | add_members | remove_members",
                    "enum": ["list", "get", "create", "update", "list_members", "add_members", "remove_members"]
                },
                "chat_id": {
                    "type": "string",
                    "description": "Chat ID (oc_xxx) — required for get, update, list_members, add_members, remove_members"
                },
                "name": {
                    "type": "string",
                    "description": "For 'create': group name (required); for 'update': new name"
                },
                "description": {
                    "type": "string",
                    "description": "For 'create'/'update': group description"
                },
                "member_open_ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "For 'create'/'add_members'/'remove_members': list of member open_ids"
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
                let (chats, next_token) = self.api.list_chats(page_token).await?;
                let mut result = json!({ "chats": chats });
                if let Some(t) = next_token {
                    result["next_page_token"] = json!(t);
                }
                Ok(result)
            }

            "get" => {
                let chat_id = args["chat_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'chat_id'".to_string()))?;
                let chat = self.api.get_chat(chat_id).await?;
                Ok(json!({ "chat": chat }))
            }

            "create" => {
                let name = args["name"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'name'".to_string()))?;
                let description = args["description"].as_str();
                let open_ids: Vec<String> = args["member_open_ids"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let chat_id = self.api.create_chat(name, description, &open_ids).await?;
                Ok(json!({ "chat_id": chat_id }))
            }

            "update" => {
                let chat_id = args["chat_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'chat_id'".to_string()))?;
                let name = args["name"].as_str();
                let description = args["description"].as_str();
                self.api.update_chat(chat_id, name, description).await?;
                Ok(json!({ "chat_id": chat_id, "status": "updated" }))
            }

            "list_members" => {
                let chat_id = args["chat_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'chat_id'".to_string()))?;
                let members = self.api.list_members(chat_id).await?;
                Ok(json!({ "members": members }))
            }

            "add_members" => {
                let chat_id = args["chat_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'chat_id'".to_string()))?;
                let open_ids: Vec<String> = args["member_open_ids"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if open_ids.is_empty() {
                    return Err(SynapticError::Tool(
                        "add_members requires 'member_open_ids'".to_string(),
                    ));
                }
                self.api.add_members(chat_id, &open_ids).await?;
                Ok(json!({ "status": "added" }))
            }

            "remove_members" => {
                let chat_id = args["chat_id"]
                    .as_str()
                    .ok_or_else(|| SynapticError::Tool("missing 'chat_id'".to_string()))?;
                let open_ids: Vec<String> = args["member_open_ids"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if open_ids.is_empty() {
                    return Err(SynapticError::Tool(
                        "remove_members requires 'member_open_ids'".to_string(),
                    ));
                }
                self.api.remove_members(chat_id, &open_ids).await?;
                Ok(json!({ "status": "removed" }))
            }

            other => Err(SynapticError::Tool(format!(
                "unknown action '{other}': expected list | get | create | update | list_members | add_members | remove_members"
            ))),
        }
    }
}
