//! Wikipedia search and summary tool.
//!
//! Uses the Wikipedia REST API — no API key required.
//! Searches Wikipedia and returns article summaries.

use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

/// A tool that searches Wikipedia and returns article summaries.
///
/// Uses the free Wikipedia REST API — no API key required.
///
/// # Example
///
/// ```rust,ignore
/// use synaptic_tools::WikipediaTool;
/// use synaptic_core::Tool;
///
/// let tool = WikipediaTool::new();
/// let result = tool.call(serde_json::json!({"query": "Rust programming language"})).await?;
/// ```
pub struct WikipediaTool {
    client: reqwest::Client,
    /// Wikipedia language code (default: `"en"`).
    language: String,
    /// Maximum number of search results to return (default: 3).
    max_results: usize,
}

impl Default for WikipediaTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WikipediaTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            language: "en".to_string(),
            max_results: 3,
        }
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }

    async fn search_titles(&self, query: &str) -> Result<Vec<String>, SynapticError> {
        let encoded_query = urlencoding::encode(query);
        let limit = self.max_results;
        let url = format!(
            "https://{lang}.wikipedia.org/w/api.php?action=query&list=search&srsearch={encoded_query}&srlimit={limit}&format=json&utf8=1",
            lang = self.language,
        );

        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "synaptic-agent/0.2 (https://github.com/dnw3/synaptic)",
            )
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("Wikipedia search failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(SynapticError::Tool(format!(
                "Wikipedia API error: HTTP {}",
                status.as_u16()
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("Wikipedia parse error: {e}")))?;

        let titles = body["query"]["search"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|r| r["title"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(titles)
    }

    async fn get_summary(&self, title: &str) -> Result<Option<Value>, SynapticError> {
        let encoded = urlencoding::encode(title);
        let url = format!(
            "https://{lang}.wikipedia.org/api/rest_v1/page/summary/{title}",
            lang = self.language,
            title = encoded,
        );

        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "synaptic-agent/0.2 (https://github.com/dnw3/synaptic)",
            )
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("Wikipedia summary request failed: {e}")))?;

        let status = response.status();
        if status.as_u16() == 404 {
            return Ok(None);
        }

        if !status.is_success() {
            return Err(SynapticError::Tool(format!(
                "Wikipedia summary error: HTTP {}",
                status.as_u16()
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("Wikipedia summary parse error: {e}")))?;

        Ok(Some(json!({
            "title": body["title"].as_str().unwrap_or(""),
            "summary": body["extract"].as_str().unwrap_or(""),
            "url": body["content_urls"]["desktop"]["page"].as_str().unwrap_or(""),
        })))
    }
}

#[async_trait]
impl Tool for WikipediaTool {
    fn name(&self) -> &'static str {
        "wikipedia_search"
    }

    fn description(&self) -> &'static str {
        "Search Wikipedia and retrieve article summaries. \
         Useful for factual questions about people, places, events, and concepts. \
         No API key required."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query or article title to look up on Wikipedia"
                }
            },
            "required": ["query"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'query' parameter".to_string()))?;

        let titles = self.search_titles(query).await?;

        if titles.is_empty() {
            return Ok(json!({
                "query": query,
                "results": [],
                "message": "No Wikipedia articles found for this query.",
            }));
        }

        let mut results = Vec::new();
        for title in &titles {
            if let Some(summary) = self.get_summary(title).await? {
                results.push(summary);
            }
        }

        Ok(json!({
            "query": query,
            "results": results,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let tool = WikipediaTool::new();
        assert_eq!(tool.name(), "wikipedia_search");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn tool_schema() {
        let tool = WikipediaTool::new();
        let schema = tool.parameters().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }

    #[test]
    fn builder_methods() {
        let tool = WikipediaTool::new().with_language("de").with_max_results(5);
        assert_eq!(tool.language, "de");
        assert_eq!(tool.max_results, 5);
    }

    #[tokio::test]
    async fn missing_query_returns_error() {
        let tool = WikipediaTool::new();
        let result = tool.call(json!({})).await;
        assert!(result.is_err());
    }
}
