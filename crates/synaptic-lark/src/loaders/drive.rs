use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::{Document, Loader, SynapticError};

use crate::{
    auth::TokenCache, loaders::doc::LarkDocLoader, loaders::spreadsheet::LarkSpreadsheetLoader,
    LarkConfig,
};

/// Load documents from a Feishu/Lark Drive folder as Synaptic [`Document`]s.
///
/// Iterates over files in the folder and loads `doc`/`docx` files via
/// [`LarkDocLoader`] and `sheet` files via [`LarkSpreadsheetLoader`].
/// Use [`recursive`] to also traverse sub-folders.
///
/// # Example
///
/// ```rust,no_run
/// use synaptic_lark::{LarkConfig, LarkDriveLoader};
/// use synaptic_core::Loader;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let loader = LarkDriveLoader::new(LarkConfig::new("cli_xxx", "secret"))
///     .with_folder_token("fldcnXxx")
///     .recursive();
/// let docs = loader.load().await?;
/// # Ok(())
/// # }
/// ```
pub struct LarkDriveLoader {
    token_cache: TokenCache,
    base_url: String,
    config_snapshot: LarkConfig,
    client: reqwest::Client,
    folder_token: Option<String>,
    recursive: bool,
}

impl LarkDriveLoader {
    /// Create a new loader using the given config.
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.clone().token_cache(),
            base_url,
            config_snapshot: config,
            client: reqwest::Client::new(),
            folder_token: None,
            recursive: false,
        }
    }

    /// Set the folder token to list files from.
    pub fn with_folder_token(mut self, t: impl Into<String>) -> Self {
        self.folder_token = Some(t.into());
        self
    }

    /// Enable recursive traversal into sub-folders.
    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }

    /// Return the folder token (empty string if not set).
    pub fn folder_token(&self) -> &str {
        self.folder_token.as_deref().unwrap_or("")
    }
}

#[async_trait]
impl Loader for LarkDriveLoader {
    async fn load(&self) -> Result<Vec<Document>, SynapticError> {
        let folder = self.folder_token.as_deref().ok_or_else(|| {
            SynapticError::Config("LarkDriveLoader: folder_token not set".to_string())
        })?;
        let token = self.token_cache.get_token().await?;
        let url = format!(
            "{}/drive/v1/files?folder_token={}&page_size=200",
            self.base_url, folder
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| SynapticError::Loader(format!("drive list: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Loader(format!("drive list parse: {e}")))?;
        if body["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Loader(format!(
                "Lark Drive API error: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )));
        }

        let mut docs = Vec::new();
        let items = body["data"]["files"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for item in &items {
            let file_type = item["type"].as_str().unwrap_or("");
            let token_val = item["token"].as_str().unwrap_or("");
            match file_type {
                "doc" | "docx" => {
                    let loader = LarkDocLoader::new(self.config_snapshot.clone())
                        .with_doc_tokens(vec![token_val.to_string()]);
                    match loader.load().await {
                        Ok(d) => docs.extend(d),
                        Err(e) => tracing::warn!("drive: skip doc {token_val}: {e}"),
                    }
                }
                "sheet" => {
                    let loader = LarkSpreadsheetLoader::new(self.config_snapshot.clone())
                        .with_token(token_val);
                    match loader.load().await {
                        Ok(d) => docs.extend(d),
                        Err(e) => tracing::warn!("drive: skip sheet {token_val}: {e}"),
                    }
                }
                "folder" if self.recursive => {
                    let sub = LarkDriveLoader::new(self.config_snapshot.clone())
                        .with_folder_token(token_val)
                        .recursive();
                    match sub.load().await {
                        Ok(d) => docs.extend(d),
                        Err(e) => tracing::warn!("drive: skip subfolder {token_val}: {e}"),
                    }
                }
                _ => {}
            }
        }
        Ok(docs)
    }
}
