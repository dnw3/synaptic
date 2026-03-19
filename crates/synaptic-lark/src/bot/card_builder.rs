//! Generic Lark Card JSON 2.0 assembly.
//!
//! Provides [`assemble_card`] to build a complete Card JSON 2.0 from
//! [`LarkCardElement`]s and a [`CardConfig`].  Business-specific elements
//! (feedback buttons, metadata footers, etc.) should be added to the
//! element list *before* calling `assemble_card`.

use serde_json::{json, Value};

use crate::card_elements::LarkCardElement;

/// Maximum elements we target after coalescing (leave room for footer).
pub const DEFAULT_MAX_ELEMENTS: usize = 180;

/// Configuration for assembling a Lark Card JSON 2.0.
#[derive(Debug, Clone)]
pub struct CardConfig {
    /// Header title text.
    pub header_title: String,
    /// Header template color (blue, green, red, etc.).
    pub template: String,
    /// Header icon token (standard_icon token, e.g. "chat_outlined"). Empty = no icon.
    pub header_icon: String,
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            header_title: String::new(),
            template: "blue".to_string(),
            header_icon: String::new(),
        }
    }
}

impl CardConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            header_title: title.into(),
            ..Default::default()
        }
    }

    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.template = template.into();
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.header_icon = icon.into();
        self
    }
}

/// Assemble a complete Lark Card JSON 2.0 from elements and config.
///
/// The caller is responsible for adding any business-specific elements
/// (feedback buttons, metadata footers) to the `elements` vec before
/// calling this function.
pub fn assemble_card(elements: Vec<LarkCardElement>, config: &CardConfig) -> Value {
    // Coalesce if over the limit
    let mut body_elements = coalesce_elements(elements, DEFAULT_MAX_ELEMENTS);

    // Convert elements to JSON objects
    let json_elements: Vec<Value> = body_elements.drain(..).map(element_to_json).collect();

    // Build header
    let mut header = json!({
        "template": &config.template,
        "title": {
            "tag": "plain_text",
            "content": &config.header_title,
        },
    });

    if !config.header_icon.is_empty() {
        header["icon"] = json!({
            "tag": "standard_icon",
            "token": &config.header_icon,
        });
    }

    json!({
        "schema": "2.0",
        "config": {
            "update_multi": true,
        },
        "header": header,
        "body": {
            "elements": json_elements,
        },
    })
}

/// Build a Card JSON 2.0 with optional streaming mode.
///
/// When `streaming` is `true`, enables client-side typewriter animation.
/// The card body contains a single markdown element with the given content.
///
/// This is used by [`super::StreamingCardWriter`] for both initial card
/// creation and final updates.
pub fn build_streaming_card(markdown_content: &str, streaming: bool, config: &CardConfig) -> Value {
    let mut card_config = json!({ "update_multi": true });
    if streaming {
        card_config["streaming_mode"] = json!(true);
        card_config["streaming_config"] = json!({
            "print_frequency_ms": { "default": 30 },
            "print_step": { "default": 2 },
            "print_strategy": "fast"
        });
    }

    let mut card = json!({
        "schema": "2.0",
        "config": card_config,
        "body": {
            "elements": [{
                "tag": "markdown",
                "content": markdown_content,
                "element_id": "streaming_content"
            }]
        }
    });

    if !config.header_title.is_empty() {
        let mut header = json!({
            "title": {
                "tag": "plain_text",
                "content": &config.header_title,
            },
            "template": &config.template,
        });
        if !config.header_icon.is_empty() {
            header["icon"] = json!({
                "tag": "standard_icon",
                "token": &config.header_icon,
            });
        }
        card["header"] = header;
    }

    card
}

/// Merge consecutive markdown elements when the count exceeds `max_elements`.
///
/// Non-markdown elements (hr, img, action, etc.) are preserved in place.
/// Consecutive markdown elements are joined with `"\n\n"` and use the
/// first element's ID.
pub fn coalesce_elements(
    elements: Vec<LarkCardElement>,
    max_elements: usize,
) -> Vec<LarkCardElement> {
    if elements.len() <= max_elements {
        return elements;
    }

    let mut result: Vec<LarkCardElement> = Vec::new();

    for elem in elements {
        if elem.tag == "markdown" {
            // Try to merge with the previous element if it is also markdown
            if let Some(last) = result.last_mut() {
                if last.tag == "markdown" {
                    let prev_content = last
                        .properties
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let curr_content = elem
                        .properties
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let merged = format!("{}\n\n{}", prev_content, curr_content);
                    last.properties["content"] = Value::String(merged);
                    continue;
                }
            }
            result.push(elem);
        } else {
            result.push(elem);
        }
    }

    result
}

/// Convert a [`LarkCardElement`] to a flat JSON object with tag, element_id,
/// and all flattened properties at the top level.
pub fn element_to_json(elem: LarkCardElement) -> Value {
    let mut obj = json!({
        "tag": elem.tag,
        "element_id": elem.element_id,
    });

    if let Value::Object(props) = elem.properties {
        if let Value::Object(ref mut map) = obj {
            for (k, v) in props {
                map.insert(k, v);
            }
        }
    }

    obj
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_md_element(id: &str, content: &str) -> LarkCardElement {
        LarkCardElement {
            tag: "markdown".to_string(),
            element_id: id.to_string(),
            properties: json!({ "content": content }),
        }
    }

    fn make_hr_element(id: &str) -> LarkCardElement {
        LarkCardElement {
            tag: "hr".to_string(),
            element_id: id.to_string(),
            properties: json!({}),
        }
    }

    #[test]
    fn test_assemble_has_schema_2() {
        let config = CardConfig::default();
        let card = assemble_card(vec![], &config);
        assert_eq!(card["schema"], "2.0");
    }

    #[test]
    fn test_assemble_has_header_with_config() {
        let config = CardConfig::new("MyBot").with_template("blue");
        let card = assemble_card(vec![], &config);
        assert_eq!(card["header"]["template"], "blue");
        assert_eq!(card["header"]["title"]["content"], "MyBot");
    }

    #[test]
    fn test_assemble_includes_body_elements() {
        let config = CardConfig::new("Bot");
        let elements = vec![
            make_md_element("e0md", "Hello world"),
            make_md_element("e1md", "Second paragraph"),
        ];
        let card = assemble_card(elements, &config);
        let body = &card["body"]["elements"];
        assert!(body.as_array().unwrap().len() >= 2);
        assert_eq!(body[0]["content"], "Hello world");
        assert_eq!(body[1]["content"], "Second paragraph");
    }

    #[test]
    fn test_assemble_update_multi_true() {
        let config = CardConfig::default();
        let card = assemble_card(vec![], &config);
        assert_eq!(card["config"]["update_multi"], true);
    }

    #[test]
    fn test_assemble_icon() {
        let config = CardConfig::new("Bot").with_icon("chat_outlined");
        let card = assemble_card(vec![], &config);
        assert_eq!(card["header"]["icon"]["token"], "chat_outlined");
    }

    #[test]
    fn test_coalesce_merges_when_over_limit() {
        let elements: Vec<LarkCardElement> = (0..200)
            .map(|i| make_md_element(&format!("e{}md", i), &format!("Para {}", i)))
            .collect();
        let coalesced = coalesce_elements(elements, 180);
        assert!(coalesced.len() <= 180);
        assert_eq!(coalesced[0].element_id, "e0md");
    }

    #[test]
    fn test_coalesce_preserves_non_markdown() {
        let elements = vec![
            make_md_element("e0", "A"),
            make_md_element("e1", "B"),
            make_hr_element("e2"),
            make_md_element("e3", "C"),
            make_md_element("e4", "D"),
        ];
        let coalesced = coalesce_elements(elements, 3);
        assert_eq!(coalesced.len(), 3);
        assert_eq!(coalesced[0].tag, "markdown");
        assert_eq!(coalesced[1].tag, "hr");
        assert_eq!(coalesced[2].tag, "markdown");
        let content = coalesced[0]
            .properties
            .get("content")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(content.contains("A"));
        assert!(content.contains("B"));
    }

    #[test]
    fn test_coalesce_no_op_under_limit() {
        let elements = vec![make_md_element("e0", "A"), make_md_element("e1", "B")];
        let coalesced = coalesce_elements(elements, 180);
        assert_eq!(coalesced.len(), 2);
    }

    #[test]
    fn test_build_streaming_card_with_streaming() {
        let config = CardConfig::new("AI").with_template("green");
        let card = build_streaming_card("Hello", true, &config);
        assert_eq!(card["config"]["streaming_mode"], true);
        assert_eq!(card["header"]["template"], "green");
    }

    #[test]
    fn test_build_streaming_card_without_streaming() {
        let config = CardConfig::new("AI");
        let card = build_streaming_card("Hello", false, &config);
        assert!(card["config"]["streaming_mode"].is_null());
    }

    #[test]
    fn test_element_to_json() {
        let elem = make_md_element("e0", "test");
        let json = element_to_json(elem);
        assert_eq!(json["tag"], "markdown");
        assert_eq!(json["element_id"], "e0");
        assert_eq!(json["content"], "test");
    }
}
