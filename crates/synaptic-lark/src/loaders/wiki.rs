use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::{Document, Loader, SynapticError};

use crate::{auth::TokenCache, LarkConfig};

/// Recursively load all documents from a Feishu/Lark Wiki space.
///
/// Traverses the wiki node tree depth-first, loading every `doc`/`docx` node
/// it finds. Use [`with_max_depth`] to limit how deep the traversal goes.
///
/// # Example
///
/// ```rust,no_run
/// use synaptic_lark::{LarkConfig, LarkWikiLoader};
/// use synaptic_core::Loader;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let loader = LarkWikiLoader::new(LarkConfig::new("cli_xxx", "secret"))
///     .with_space_id("space_xxx")
///     .with_max_depth(3);
/// let docs = loader.load().await?;
/// # Ok(())
/// # }
/// ```
pub struct LarkWikiLoader {
    token_cache: TokenCache,
    base_url: String,
    client: reqwest::Client,
    space_id: Option<String>,
    max_depth: Option<usize>,
    config_snapshot: LarkConfig,
}

impl LarkWikiLoader {
    /// Create a new loader using the given config.
    pub fn new(config: LarkConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            token_cache: config.clone().token_cache(),
            base_url,
            client: reqwest::Client::new(),
            space_id: None,
            max_depth: None,
            config_snapshot: config,
        }
    }

    /// Set the Wiki space ID to traverse.
    pub fn with_space_id(mut self, id: impl Into<String>) -> Self {
        self.space_id = Some(id.into());
        self
    }

    /// Limit recursive traversal to `d` levels (default: unlimited).
    pub fn with_max_depth(mut self, d: usize) -> Self {
        self.max_depth = Some(d);
        self
    }

    /// Return the space ID (empty string if not set).
    pub fn space_id(&self) -> &str {
        self.space_id.as_deref().unwrap_or("")
    }

    /// Return the configured max depth.
    pub fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }

    /// Recursively collect all document/docx obj_tokens under a wiki space node.
    async fn collect_tokens(
        &self,
        token: &str,
        parent_node_token: Option<&str>,
        depth: usize,
    ) -> Result<Vec<String>, SynapticError> {
        if let Some(max) = self.max_depth {
            if depth > max {
                return Ok(vec![]);
            }
        }
        let space_id = self.space_id.as_deref().unwrap();
        let mut url = format!(
            "{}/wiki/v2/spaces/{}/nodes?page_size=50",
            self.base_url, space_id
        );
        if let Some(pt) = parent_node_token {
            url.push_str(&format!("&parent_node_token={pt}"));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SynapticError::Loader(format!("wiki nodes: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Loader(format!("wiki nodes parse: {e}")))?;
        if body["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Loader(format!(
                "Lark Wiki API error: {}",
                body["msg"].as_str().unwrap_or("unknown")
            )));
        }

        let mut tokens = vec![];
        let items = body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for item in &items {
            let obj_type = item["obj_type"].as_str().unwrap_or("");
            let obj_token = item["obj_token"].as_str().unwrap_or("").to_string();
            let node_token = item["node_token"].as_str().unwrap_or("");
            let has_child = item["has_child"].as_bool().unwrap_or(false);

            if obj_type == "doc" || obj_type == "docx" {
                tokens.push(obj_token);
            }
            if has_child {
                let children =
                    Box::pin(self.collect_tokens(token, Some(node_token), depth + 1)).await?;
                tokens.extend(children);
            }
        }
        Ok(tokens)
    }
}

#[async_trait]
impl Loader for LarkWikiLoader {
    async fn load(&self) -> Result<Vec<Document>, SynapticError> {
        self.space_id
            .as_deref()
            .ok_or_else(|| SynapticError::Config("LarkWikiLoader: space_id not set".to_string()))?;
        let token = self.token_cache.get_token().await?;
        let doc_tokens = self.collect_tokens(&token, None, 0).await?;

        let loader = crate::loaders::doc::LarkDocLoader::new(self.config_snapshot.clone())
            .with_doc_tokens(doc_tokens);
        loader.load().await
    }
}
