//! DuckDuckGo Instant Answer search tool.
//!
//! Uses the free DuckDuckGo Instant Answer API — no API key required.
//! Returns top results from DuckDuckGo instant answer, related topics, and web results.

use async_trait::async_trait;
use serde_json::{json, Value};
use synaptic_core::{SynapticError, Tool};

/// A search tool powered by the DuckDuckGo Instant Answer API.
///
/// No API key is required. Results include the abstract (featured snippet),
/// related topics, and answer when available.
///
/// # Example
///
/// ```rust,ignore
/// use synaptic_tools::DuckDuckGoTool;
/// use synaptic_core::Tool;
///
/// let tool = DuckDuckGoTool::new();
/// let result = tool.call(serde_json::json!({"query": "Rust programming"})).await?;
/// ```
pub struct DuckDuckGoTool {
    client: reqwest::Client,
    /// Maximum number of related topics to include in results (default: 5).
    max_results: usize,
}

impl Default for DuckDuckGoTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckDuckGoTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            max_results: 5,
        }
    }

    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }
}

#[async_trait]
impl Tool for DuckDuckGoTool {
    fn name(&self) -> &'static str {
        "duckduckgo_search"
    }

    fn description(&self) -> &'static str {
        "Search the web using DuckDuckGo. Returns instant answers, featured snippets, \
         and related topics. No API key required."
    }

    fn parameters(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        }))
    }

    async fn call(&self, args: Value) -> Result<Value, SynapticError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| SynapticError::Tool("missing 'query' parameter".to_string()))?;

        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "https://api.duckduckgo.com/?q={encoded_query}&format=json&no_html=1&skip_disambig=1&no_redirect=1"
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "synaptic-agent/0.2")
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("DuckDuckGo request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Err(SynapticError::Tool(format!(
                "DuckDuckGo API error: HTTP {status}"
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("DuckDuckGo parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(abstract_text) = body["Abstract"].as_str() {
            if !abstract_text.is_empty() {
                results.push(json!({
                    "type": "abstract",
                    "title": body["Heading"].as_str().unwrap_or(""),
                    "snippet": abstract_text,
                    "url": body["AbstractURL"].as_str().unwrap_or(""),
                    "source": body["AbstractSource"].as_str().unwrap_or(""),
                }));
            }
        }

        if let Some(answer) = body["Answer"].as_str() {
            if !answer.is_empty() {
                results.push(json!({
                    "type": "answer",
                    "snippet": answer,
                    "answer_type": body["AnswerType"].as_str().unwrap_or(""),
                }));
            }
        }

        if let Some(topics) = body["RelatedTopics"].as_array() {
            let mut count = 0;
            for topic in topics {
                if count >= self.max_results {
                    break;
                }
                if let Some(text) = topic["Text"].as_str() {
                    if !text.is_empty() {
                        results.push(json!({
                            "type": "related",
                            "snippet": text,
                            "url": topic["FirstURL"].as_str().unwrap_or(""),
                        }));
                        count += 1;
                    }
                }
            }
        }

        if results.is_empty() {
            return Ok(json!({
                "query": query,
                "results": [],
                "message": "No results found. Try a more specific query.",
            }));
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
        let tool = DuckDuckGoTool::new();
        assert_eq!(tool.name(), "duckduckgo_search");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn tool_schema() {
        let tool = DuckDuckGoTool::new();
        let schema = tool.parameters().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }

    #[tokio::test]
    async fn missing_query_returns_error() {
        let tool = DuckDuckGoTool::new();
        let result = tool.call(json!({})).await;
        assert!(result.is_err());
    }
}
