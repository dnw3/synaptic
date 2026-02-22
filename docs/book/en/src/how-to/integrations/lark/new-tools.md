# Productivity Tools

Five new Agent tools for managing Feishu contacts, chats, spreadsheets, calendar events, and tasks.

---

## LarkContactTool

Look up Feishu users and departments.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkContactTool};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkContactTool::new(config.clone());

// Get user by open_id
tool.call(json!({
    "action": "get_user",
    "user_id": "ou_xxx",
    "user_id_type": "open_id"
})).await?;

// Batch resolve emails to open_ids
tool.call(json!({
    "action": "batch_get_id",
    "emails": ["user@example.com"]
})).await?;

// List departments
tool.call(json!({ "action": "list_departments" })).await?;
```

### Actions

| Action | Required fields | Description |
|--------|----------------|-------------|
| `get_user` | `user_id` | Get a user by ID (default type: `open_id`) |
| `batch_get_id` | `emails` or `mobiles` | Resolve emails/mobiles to open_ids |
| `list_departments` | — | List departments (optional `parent_department_id`) |
| `get_department` | `department_id` | Get a department by ID |

---

## LarkChatTool

Manage group chats — list, create, update membership.

```rust,ignore
use synaptic::lark::{LarkChatTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkChatTool::new(config.clone());

// List all chats the bot is in
tool.call(json!({ "action": "list" })).await?;

// Create a group with members
tool.call(json!({
    "action": "create",
    "name": "Project Alpha",
    "member_open_ids": ["ou_xxx"]
})).await?;

// Add members to existing group
tool.call(json!({
    "action": "add_members",
    "chat_id": "oc_xxx",
    "member_open_ids": ["ou_yyy"]
})).await?;
```

### Actions

| Action | Required fields | Description |
|--------|----------------|-------------|
| `list` | — | List all chats the bot belongs to |
| `get` | `chat_id` | Get details of a specific chat |
| `create` | `name` | Create a group chat (optional: `description`, `member_open_ids`) |
| `update` | `chat_id` | Update name or description |
| `list_members` | `chat_id` | List members of a chat |
| `add_members` | `chat_id`, `member_open_ids` | Add members |
| `remove_members` | `chat_id`, `member_open_ids` | Remove members |

---

## LarkSpreadsheetTool

Read and write Feishu Spreadsheet ranges.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkSpreadsheetTool};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkSpreadsheetTool::new(config.clone());

// Write data to a range
tool.call(json!({
    "action": "write",
    "spreadsheet_token": "shtcnXxx",
    "range": "Sheet1!A1:B2",
    "values": [["Name", "Score"], ["Alice", 95]]
})).await?;

// Append rows
tool.call(json!({
    "action": "append",
    "spreadsheet_token": "shtcnXxx",
    "range": "Sheet1!A:B",
    "values": [["Bob", 88]]
})).await?;

// Read a range
tool.call(json!({
    "action": "read",
    "spreadsheet_token": "shtcnXxx",
    "range": "Sheet1!A1:B3"
})).await?;
```

Range format: `"SheetName!A1:B3"` (same notation as Google Sheets).

### Actions

| Action | Required fields | Description |
|--------|----------------|-------------|
| `write` | `spreadsheet_token`, `range`, `values` | Overwrite a range with 2D array |
| `append` | `spreadsheet_token`, `range`, `values` | Append rows after last row |
| `clear` | `spreadsheet_token`, `range` | Clear all values in a range |
| `read` | `spreadsheet_token`, `range` | Read values; returns `{ values: [[...], ...] }` |

---

## LarkCalendarTool

Manage calendar events — create, list, update, delete.

```rust,ignore
use synaptic::lark::{LarkCalendarTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkCalendarTool::new(config.clone());

// Create an event
tool.call(json!({
    "action": "create_event",
    "calendar_id": "primary",
    "summary": "Team Sync",
    "start_time": "1735689600",
    "end_time": "1735693200",
    "description": "Weekly sync"
})).await?;

// List upcoming events
tool.call(json!({
    "action": "list_events",
    "calendar_id": "primary",
    "start_time": "1735689600"
})).await?;
```

Times are Unix timestamp strings (seconds since epoch).

### Actions

| Action | Required fields | Description |
|--------|----------------|-------------|
| `list_calendars` | — | List accessible calendars |
| `list_events` | `calendar_id` | List events (optional `start_time`, `end_time` filter) |
| `get_event` | `calendar_id`, `event_id` | Get event details |
| `create_event` | `calendar_id`, `summary`, `start_time`, `end_time` | Create an event |
| `update_event` | `calendar_id`, `event_id` | Update event fields (optional: `summary`, `description`, `start_time`, `end_time`) |
| `delete_event` | `calendar_id`, `event_id` | Delete an event |

---

## LarkTaskTool

Create and manage Feishu Tasks.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkTaskTool};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkTaskTool::new(config.clone());

// Create a task
tool.call(json!({
    "action": "create",
    "summary": "Review PR #42",
    "due_timestamp": "1735689600"
})).await?;

// Complete a task
tool.call(json!({
    "action": "complete",
    "task_guid": "task_xxx"
})).await?;

// List tasks
tool.call(json!({ "action": "list" })).await?;
```

### Actions

| Action | Required fields | Description |
|--------|----------------|-------------|
| `list` | — | List tasks (paginated, optional `page_token`) |
| `get` | `task_guid` | Get task details |
| `create` | `summary` | Create a task (optional: `due_timestamp`, `description`) |
| `update` | `task_guid` | Update task fields (optional: `summary`, `description`, `due_timestamp`) |
| `complete` | `task_guid` | Mark a task as complete |
| `delete` | `task_guid` | Delete a task |
