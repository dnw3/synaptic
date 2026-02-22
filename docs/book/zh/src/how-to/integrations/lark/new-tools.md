# 效率工具

五个全新 Agent 工具，用于管理飞书通讯录、群聊、电子表格、日历事件和任务。

---

## LarkContactTool

查询飞书用户和部门信息。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkContactTool};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkContactTool::new(config.clone());

// 通过 open_id 查询用户
tool.call(json!({
    "action": "get_user",
    "user_id": "ou_xxx",
    "user_id_type": "open_id"
})).await?;

// 批量将邮箱解析为 open_id
tool.call(json!({
    "action": "batch_get_id",
    "emails": ["user@example.com"]
})).await?;

// 列出部门
tool.call(json!({ "action": "list_departments" })).await?;
```

### 操作说明

| 操作 | 必填字段 | 说明 |
|------|---------|------|
| `get_user` | `user_id` | 根据 ID 查询用户（默认类型：`open_id`） |
| `batch_get_id` | `emails` 或 `mobiles` | 将邮箱/手机号解析为 open_id |
| `list_departments` | — | 列出部门（可选 `parent_department_id`） |
| `get_department` | `department_id` | 根据 ID 查询部门 |

---

## LarkChatTool

管理群聊——列出、创建、更新成员。

```rust,ignore
use synaptic::lark::{LarkChatTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkChatTool::new(config.clone());

// 列出机器人加入的所有群
tool.call(json!({ "action": "list" })).await?;

// 创建群并添加成员
tool.call(json!({
    "action": "create",
    "name": "项目 Alpha",
    "member_open_ids": ["ou_xxx"]
})).await?;

// 向已有群添加成员
tool.call(json!({
    "action": "add_members",
    "chat_id": "oc_xxx",
    "member_open_ids": ["ou_yyy"]
})).await?;
```

### 操作说明

| 操作 | 必填字段 | 说明 |
|------|---------|------|
| `list` | — | 列出机器人所在的所有群 |
| `get` | `chat_id` | 查询群详情 |
| `create` | `name` | 创建群聊（可选：`description`、`member_open_ids`） |
| `update` | `chat_id` | 更新群名称或描述 |
| `list_members` | `chat_id` | 列出群成员 |
| `add_members` | `chat_id`, `member_open_ids` | 添加成员 |
| `remove_members` | `chat_id`, `member_open_ids` | 移除成员 |

---

## LarkSpreadsheetTool

读写飞书电子表格范围数据。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkSpreadsheetTool};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkSpreadsheetTool::new(config.clone());

// 写入数据到指定范围
tool.call(json!({
    "action": "write",
    "spreadsheet_token": "shtcnXxx",
    "range": "Sheet1!A1:B2",
    "values": [["姓名", "分数"], ["Alice", 95]]
})).await?;

// 追加行
tool.call(json!({
    "action": "append",
    "spreadsheet_token": "shtcnXxx",
    "range": "Sheet1!A:B",
    "values": [["Bob", 88]]
})).await?;

// 读取范围数据
tool.call(json!({
    "action": "read",
    "spreadsheet_token": "shtcnXxx",
    "range": "Sheet1!A1:B3"
})).await?;
```

范围格式：`"SheetName!A1:B3"`（与 Google Sheets 相同）。

### 操作说明

| 操作 | 必填字段 | 说明 |
|------|---------|------|
| `write` | `spreadsheet_token`, `range`, `values` | 覆写指定范围（二维数组） |
| `append` | `spreadsheet_token`, `range`, `values` | 在最后一行之后追加行 |
| `clear` | `spreadsheet_token`, `range` | 清空范围内的数据 |
| `read` | `spreadsheet_token`, `range` | 读取数据，返回 `{ values: [[...], ...] }` |

---

## LarkCalendarTool

管理日历事件——创建、列出、更新、删除。

```rust,ignore
use synaptic::lark::{LarkCalendarTool, LarkConfig};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkCalendarTool::new(config.clone());

// 创建日程
tool.call(json!({
    "action": "create_event",
    "calendar_id": "primary",
    "summary": "周同步会议",
    "start_time": "1735689600",
    "end_time": "1735693200",
    "description": "每周同步"
})).await?;

// 查询即将到来的日程
tool.call(json!({
    "action": "list_events",
    "calendar_id": "primary",
    "start_time": "1735689600"
})).await?;
```

时间格式为 Unix 时间戳字符串（秒）。

### 操作说明

| 操作 | 必填字段 | 说明 |
|------|---------|------|
| `list_calendars` | — | 列出可访问的日历 |
| `list_events` | `calendar_id` | 列出日程（可选 `start_time`、`end_time` 过滤） |
| `get_event` | `calendar_id`, `event_id` | 查询日程详情 |
| `create_event` | `calendar_id`, `summary`, `start_time`, `end_time` | 新建日程 |
| `update_event` | `calendar_id`, `event_id` | 更新日程字段（可选：`summary`、`description`、`start_time`、`end_time`） |
| `delete_event` | `calendar_id`, `event_id` | 删除日程 |

---

## LarkTaskTool

创建和管理飞书任务。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkTaskTool};
use synaptic::core::Tool;
use serde_json::json;

let tool = LarkTaskTool::new(config.clone());

// 创建任务
tool.call(json!({
    "action": "create",
    "summary": "审查 PR #42",
    "due_timestamp": "1735689600"
})).await?;

// 完成任务
tool.call(json!({
    "action": "complete",
    "task_guid": "task_xxx"
})).await?;

// 列出任务
tool.call(json!({ "action": "list" })).await?;
```

### 操作说明

| 操作 | 必填字段 | 说明 |
|------|---------|------|
| `list` | — | 列出任务（分页，可选 `page_token`） |
| `get` | `task_guid` | 查询任务详情 |
| `create` | `summary` | 新建任务（可选：`due_timestamp`、`description`） |
| `update` | `task_guid` | 更新任务字段（可选：`summary`、`description`、`due_timestamp`） |
| `complete` | `task_guid` | 将任务标记为已完成 |
| `delete` | `task_guid` | 删除任务 |
