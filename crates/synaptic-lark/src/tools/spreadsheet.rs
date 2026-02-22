use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

use crate::{api::spreadsheet::SpreadsheetApi, LarkConfig};

/// Read and write Feishu/Lark Spreadsheet ranges as an Agent tool.
///
/// # Actions
///
/// | Action   | Description                                      |
/// |----------|--------------------------------------------------|
/// | `write`  | Overwrite a range with new values                |
/// | `append` | Append rows after the last row in a range        |
/// | `clear`  | Clear all values in a range                      |
/// | `read`   | Read values from a range                         |
///
/// Range format: `"SheetName!A1:B3"` (same as Google Sheets notation).
pub struct LarkSpreadsheetTool {
    api: SpreadsheetApi,
}

impl LarkSpreadsheetTool {
    /// Create a new spreadsheet tool.
    pub fn new(config: LarkConfig) -> Self {
        Self {
            api: SpreadsheetApi::new(config),
        }
    }
}

#[async_trait]
impl Tool for LarkSpreadsheetTool {
    fn name(&self) -> &'static str {
        "lark_spreadsheet"
    }

    fn description(&self) -> &'static str {
        "Read and write Feishu/Lark Spreadsheet data. \
         Use action='write' to overwrite a range; \
         action='append' to add rows at the end; \
         action='clear' to clear a range; \
         action='read' to read values from a range. \
         Range format: 'SheetName!A1:B3'."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: write | append | clear | read",
                    "enum": ["write", "append", "clear", "read"]
                },
                "spreadsheet_token": {
                    "type": "string",
                    "description": "The spreadsheet token (e.g. shtcnXxx)"
                },
                "range": {
                    "type": "string",
                    "description": "Cell range in 'SheetName!A1:B3' format"
                },
                "values": {
                    "type": "array",
                    "items": {
                        "type": "array",
                        "items": {}
                    },
                    "description": "For 'write'/'append': 2D array of values (rows × columns)"
                }
            },
            "required": ["action"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'action'".to_string()))?;

        // Validate action first before requiring common parameters
        match action {
            "write" | "append" | "clear" | "read" => {}
            other => {
                return Err(SynapticError::Tool(format!(
                    "unknown action '{other}': expected write | append | clear | read"
                )));
            }
        }

        let spreadsheet_token = args["spreadsheet_token"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'spreadsheet_token'".to_string()))?;

        let range = args["range"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'range'".to_string()))?;

        match action {
            "write" => {
                let values = parse_values(&args)?;
                self.api
                    .write_values(spreadsheet_token, range, values)
                    .await?;
                Ok(json!({ "status": "written" }))
            }

            "append" => {
                let values = parse_values(&args)?;
                self.api
                    .append_values(spreadsheet_token, range, values)
                    .await?;
                Ok(json!({ "status": "appended" }))
            }

            "clear" => {
                self.api.clear_values(spreadsheet_token, range).await?;
                Ok(json!({ "status": "cleared" }))
            }

            "read" => {
                let values = self.api.read_values(spreadsheet_token, range).await?;
                Ok(json!({ "values": values }))
            }

            // Already validated above; this branch is unreachable
            _ => unreachable!(),
        }
    }
}

fn parse_values(args: &Value) -> Result<Vec<Vec<Value>>, SynapticError> {
    args["values"]
        .as_array()
        .ok_or_else(|| SynapticError::Tool("missing 'values'".to_string()))?
        .iter()
        .map(|row| {
            row.as_array()
                .map(|r| r.to_vec())
                .ok_or_else(|| SynapticError::Tool("'values' must be a 2D array".to_string()))
        })
        .collect()
}
