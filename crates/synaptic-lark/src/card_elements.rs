//! Lark Card JSON 2.0 element types and rendering from MessageIR.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use synaptic_core::message_ir::{
    apply_lark_md_spans, escape_json_string, render_table_md, Block, MessageIR, RenderOptions,
};

/// A structured Lark Card JSON 2.0 element for embedding in card body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkCardElement {
    pub tag: String,
    pub element_id: String,
    #[serde(flatten)]
    pub properties: Value,
}

/// Convert a [`MessageIR`] into structured Lark Card 2.0 elements.
///
/// Each IR [`Block`] becomes one independent card element. This produces
/// v2.0-style elements (`markdown` + `content`) rather than v1 (`div` + `lark_md`).
pub fn render_lark_card_elements(ir: &MessageIR, options: &RenderOptions) -> Vec<LarkCardElement> {
    let mut elements = Vec::new();

    for (idx, block) in ir.blocks.iter().enumerate() {
        match block {
            Block::Heading { level, text } => {
                let text_size = match level {
                    1 => "heading-1",
                    2 => "heading-2",
                    3 => "heading-3",
                    4 => "heading-4",
                    _ => "normal",
                };
                let content = apply_lark_md_spans(text);
                elements.push(LarkCardElement {
                    tag: "markdown".to_string(),
                    element_id: format!("e{}md", idx),
                    properties: serde_json::json!({
                        "content": content,
                        "text_size": text_size,
                    }),
                });
            }
            Block::Paragraph(rt) => {
                let content = apply_lark_md_spans(rt);
                elements.push(LarkCardElement {
                    tag: "markdown".to_string(),
                    element_id: format!("e{}md", idx),
                    properties: serde_json::json!({
                        "content": content,
                    }),
                });
            }
            Block::CodeBlock { language, code } => {
                let lang = language.as_deref().unwrap_or("");
                let content = format!("```{}\n{}\n```", lang, code);
                elements.push(LarkCardElement {
                    tag: "markdown".to_string(),
                    element_id: format!("e{}md", idx),
                    properties: serde_json::json!({
                        "content": content,
                    }),
                });
            }
            Block::List { ordered, items } => {
                let mut lines = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    if *ordered {
                        lines.push(format!("{}. {}", i + 1, apply_lark_md_spans(item)));
                    } else {
                        lines.push(format!("- {}", apply_lark_md_spans(item)));
                    }
                }
                let content = lines.join("\n");
                elements.push(LarkCardElement {
                    tag: "markdown".to_string(),
                    element_id: format!("e{}md", idx),
                    properties: serde_json::json!({
                        "content": content,
                    }),
                });
            }
            Block::Blockquote(rt) => {
                let quoted: String = apply_lark_md_spans(rt)
                    .lines()
                    .map(|l| format!("> {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");
                elements.push(LarkCardElement {
                    tag: "markdown".to_string(),
                    element_id: format!("e{}md", idx),
                    properties: serde_json::json!({
                        "content": quoted,
                    }),
                });
            }
            Block::Table { headers, rows } => {
                let mut table_md = String::new();
                render_table_md(&mut table_md, headers, rows, options.table_mode);
                let content = table_md.trim().to_string();
                elements.push(LarkCardElement {
                    tag: "markdown".to_string(),
                    element_id: format!("e{}md", idx),
                    properties: serde_json::json!({
                        "content": content,
                    }),
                });
            }
            Block::ThematicBreak => {
                elements.push(LarkCardElement {
                    tag: "hr".to_string(),
                    element_id: format!("e{}hr", idx),
                    properties: serde_json::json!({}),
                });
            }
            Block::Image { alt, url } => {
                if !options.preserve_images {
                    continue;
                }
                if url.starts_with("img_v") {
                    // Native Lark image key
                    elements.push(LarkCardElement {
                        tag: "img".to_string(),
                        element_id: format!("e{}img", idx),
                        properties: serde_json::json!({
                            "img_key": url,
                            "alt": { "tag": "plain_text", "content": alt },
                        }),
                    });
                } else {
                    // Fallback: render as markdown image
                    let content = format!("![{}]({})", escape_json_string(alt), url);
                    elements.push(LarkCardElement {
                        tag: "markdown".to_string(),
                        element_id: format!("e{}md", idx),
                        properties: serde_json::json!({
                            "content": content,
                        }),
                    });
                }
            }
        }
    }

    elements
}
