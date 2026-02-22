use crate::{auth::TokenCache, LarkConfig};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use synaptic_core::{Document, Loader, SynapticError};

/// Load rows from a Feishu/Lark spreadsheet as Synaptic [`Document`]s.
///
/// Each non-header row becomes one document. The column designated by
/// [`with_content_col`] supplies the document `content`; all other columns
/// are stored in the document `metadata`.
///
/// # Example
///
/// ```rust,no_run
/// use synaptic_lark::{LarkConfig, LarkSpreadsheetLoader};
/// use synaptic_core::Loader;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let loader = LarkSpreadsheetLoader::new(LarkConfig::new("cli_xxx", "secret"))
///     .with_token("shtcnXxx")
///     .with_sheet("0")
///     .with_content_col(0)
///     .with_header_row(true);
/// let docs = loader.load().await?;
/// # Ok(())
/// # }
/// ```
pub struct LarkSpreadsheetLoader {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
    spreadsheet_token: Option<String>,
    sheet_id: String,
    content_col: usize,
    header_row: bool,
}

impl LarkSpreadsheetLoader {
    /// Create a new loader using the given config.
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.token_cache(),
            base_url,
            client: reqwest::Client::new(),
            spreadsheet_token: None,
            sheet_id: "0".to_string(),
            content_col: 0,
            header_row: true,
        }
    }

    /// Set the spreadsheet token (e.g. `"shtcnXxx"`).
    pub fn with_token(mut self, t: impl Into<String>) -> Self {
        self.spreadsheet_token = Some(t.into());
        self
    }

    /// Set the sheet ID within the spreadsheet (default `"0"`).
    pub fn with_sheet(mut self, id: impl Into<String>) -> Self {
        self.sheet_id = id.into();
        self
    }

    /// Set which column (0-indexed) to use as document `content` (default `0`).
    pub fn with_content_col(mut self, col: usize) -> Self {
        self.content_col = col;
        self
    }

    /// Whether the first row is a header row (default `true`).
    pub fn with_header_row(mut self, v: bool) -> Self {
        self.header_row = v;
        self
    }

    /// Return the spreadsheet token (empty string if not set).
    pub fn spreadsheet_token(&self) -> &str {
        self.spreadsheet_token.as_deref().unwrap_or("")
    }

    /// Return the sheet ID.
    pub fn sheet_id(&self) -> &str {
        &self.sheet_id
    }
}

#[async_trait]
impl Loader for LarkSpreadsheetLoader {
    async fn load(&self) -> Result<Vec<Document>, SynapticError> {
        let stoken = self.spreadsheet_token.as_deref().ok_or_else(|| {
            SynapticError::Config("LarkSpreadsheetLoader: spreadsheet_token not set".to_string())
        })?;
        let token = self.token_cache.get_token().await?;

        let range = format!("{}!A1:ZZ10000", self.sheet_id);
        let url = format!(
            "{}/sheets/v2/spreadsheets/{}/values/{}?renderType=PlainText",
            self.base_url,
            stoken,
            urlencoding::encode(&range)
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Loader(format!("spreadsheet fetch: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Loader(format!("spreadsheet parse: {e}")))?;
        if body["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Loader(format!(
                "Lark Spreadsheet API error: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )));
        }

        let rows = body["data"]["valueRange"]["values"]
            .as_array()
            .ok_or_else(|| SynapticError::Loader("no values in spreadsheet".to_string()))?;

        let empty_row: Vec<Value> = Vec::new();
        let (headers, data_rows) = if self.header_row && !rows.is_empty() {
            let hdrs: Vec<String> = rows[0]
                .as_array()
                .unwrap_or(&empty_row)
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect();
            (hdrs, &rows[1..])
        } else {
            (vec![], rows.as_slice())
        };

        let empty: Vec<Value> = Vec::new();
        let mut docs = Vec::new();
        for (i, row) in data_rows.iter().enumerate() {
            let cells = row.as_array().unwrap_or(&empty);
            let content = cells
                .get(self.content_col)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut metadata = HashMap::new();
            metadata.insert(
                "row_index".to_string(),
                Value::from(i + if self.header_row { 2 } else { 1 }),
            );
            metadata.insert(
                "source".to_string(),
                Value::String("lark_spreadsheet".to_string()),
            );
            for (j, cell) in cells.iter().enumerate() {
                if j == self.content_col {
                    continue;
                }
                let col_name = headers
                    .get(j)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{j}"));
                metadata.insert(col_name, cell.clone());
            }

            docs.push(Document {
                id: format!("{}_{}", stoken, i),
                content,
                metadata,
            });
        }
        Ok(docs)
    }
}
