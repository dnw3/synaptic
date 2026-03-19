//! Lark-specific IR renderers.
//!
//! - [`LarkCardIRRenderer`] — renders IR to Lark Card v1 JSON element array.
//! - [`LarkTextIRRenderer`] — renders IR to plain text for Lark text messages.

use synaptic_core::message_ir::{
    apply_lark_md_spans, escape_json_string, render_table_md, Block, IRRenderer, MessageIR,
    PlainTextRenderer, RenderOptions,
};

/// Renders IR to Lark interactive card v1 JSON (array of elements).
pub struct LarkCardIRRenderer;

impl IRRenderer for LarkCardIRRenderer {
    fn render(&self, ir: &MessageIR, options: &RenderOptions) -> String {
        let mut elements: Vec<String> = Vec::new();

        for block in &ir.blocks {
            match block {
                Block::Paragraph(rt) => {
                    let content = escape_json_string(&apply_lark_md_spans(rt));
                    elements.push(format!(
                        r#"{{"tag":"div","text":{{"tag":"lark_md","content":"{}"}}}}"#,
                        content
                    ));
                }
                Block::CodeBlock { language, code } => {
                    let lang = language.as_deref().unwrap_or("");
                    let fenced = format!("```{}\n{}\n```", lang, code);
                    let content = escape_json_string(&fenced);
                    elements.push(format!(
                        r#"{{"tag":"div","text":{{"tag":"lark_md","content":"{}"}}}}"#,
                        content
                    ));
                }
                Block::Heading { level, text } => {
                    let prefix = "#".repeat(*level as usize);
                    let content =
                        escape_json_string(&format!("{} {}", prefix, apply_lark_md_spans(text)));
                    elements.push(format!(
                        r#"{{"tag":"div","text":{{"tag":"lark_md","content":"{}"}}}}"#,
                        content
                    ));
                }
                Block::List { ordered, items } => {
                    let mut lines = Vec::new();
                    for (idx, item) in items.iter().enumerate() {
                        if *ordered {
                            lines.push(format!("{}. {}", idx + 1, apply_lark_md_spans(item)));
                        } else {
                            lines.push(format!("- {}", apply_lark_md_spans(item)));
                        }
                    }
                    let content = escape_json_string(&lines.join("\n"));
                    elements.push(format!(
                        r#"{{"tag":"div","text":{{"tag":"lark_md","content":"{}"}}}}"#,
                        content
                    ));
                }
                Block::Blockquote(rt) => {
                    let quoted: String = apply_lark_md_spans(rt)
                        .lines()
                        .map(|l| format!("> {}", l))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let content = escape_json_string(&quoted);
                    elements.push(format!(
                        r#"{{"tag":"div","text":{{"tag":"lark_md","content":"{}"}}}}"#,
                        content
                    ));
                }
                Block::Table { headers, rows } => {
                    // Render as lark_md table
                    let mut table_md = String::new();
                    render_table_md(&mut table_md, headers, rows, options.table_mode);
                    let content = escape_json_string(table_md.trim());
                    elements.push(format!(
                        r#"{{"tag":"div","text":{{"tag":"lark_md","content":"{}"}}}}"#,
                        content
                    ));
                }
                Block::ThematicBreak => {
                    elements.push(r#"{"tag":"hr"}"#.to_string());
                }
                Block::Image { url, .. } => {
                    if options.preserve_images {
                        let url_escaped = escape_json_string(url);
                        elements.push(format!(r#"{{"tag":"img","img_key":"{}"}}"#, url_escaped));
                    }
                }
            }
        }

        format!("[{}]", elements.join(","))
    }
}

/// Renders IR to plain text for Lark text messages (delegates to [`PlainTextRenderer`]).
pub struct LarkTextIRRenderer;

impl IRRenderer for LarkTextIRRenderer {
    fn render(&self, ir: &MessageIR, options: &RenderOptions) -> String {
        PlainTextRenderer.render(ir, options)
    }
}
