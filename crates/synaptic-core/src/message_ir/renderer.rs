use super::*;

/// Render an IR to the specified target format.
pub fn render(ir: &MessageIR, options: &RenderOptions) -> String {
    match options.target {
        RenderTarget::Markdown => render_markdown(ir, options),
        RenderTarget::LarkCard => render_lark_card(ir, options),
        RenderTarget::LarkText => render_plain(ir, options),
        RenderTarget::SlackMrkdwn => render_slack(ir, options),
        RenderTarget::TelegramHtml => render_telegram(ir, options),
        RenderTarget::SignalRanges => render_plain(ir, options),
        RenderTarget::PlainText => render_plain(ir, options),
    }
}

// ===========================================================================
// Markdown renderer
// ===========================================================================

fn render_markdown(ir: &MessageIR, options: &RenderOptions) -> String {
    let mut out = String::new();
    for (i, block) in ir.blocks.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match block {
            Block::Paragraph(rt) => {
                out.push_str(&apply_md_spans(rt));
                out.push('\n');
            }
            Block::CodeBlock { language, code } => {
                out.push_str("```");
                if let Some(lang) = language {
                    out.push_str(lang);
                }
                out.push('\n');
                out.push_str(code);
                out.push_str("\n```\n");
            }
            Block::Heading { level, text } => {
                for _ in 0..*level {
                    out.push('#');
                }
                out.push(' ');
                out.push_str(&apply_md_spans(text));
                out.push('\n');
            }
            Block::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    if *ordered {
                        out.push_str(&format!("{}. ", idx + 1));
                    } else {
                        out.push_str("- ");
                    }
                    out.push_str(&apply_md_spans(item));
                    out.push('\n');
                }
            }
            Block::Blockquote(rt) => {
                for line in apply_md_spans(rt).lines() {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            Block::Table { headers, rows } => {
                render_table_md(&mut out, headers, rows, options.table_mode);
            }
            Block::ThematicBreak => {
                out.push_str("---\n");
            }
            Block::Image { alt, url } => {
                if options.preserve_images {
                    out.push_str(&format!("![{}]({})\n", alt, url));
                }
            }
        }
    }
    out
}

pub fn render_table_md(
    out: &mut String,
    headers: &[String],
    rows: &[Vec<String>],
    mode: TableMode,
) {
    match mode {
        TableMode::Code => {
            // Render as a markdown table
            out.push('|');
            for h in headers {
                out.push_str(&format!(" {} |", h));
            }
            out.push('\n');
            out.push('|');
            for _ in headers {
                out.push_str(" --- |");
            }
            out.push('\n');
            for row in rows {
                out.push('|');
                for cell in row {
                    out.push_str(&format!(" {} |", cell));
                }
                out.push('\n');
            }
        }
        TableMode::Bullets => {
            for row in rows {
                out.push_str("- ");
                let pairs: Vec<String> = headers
                    .iter()
                    .zip(row.iter())
                    .map(|(h, c)| format!("{}: {}", h, c))
                    .collect();
                out.push_str(&pairs.join(", "));
                out.push('\n');
            }
        }
        TableMode::Off => {
            // Just dump cells separated by tabs
            for h in headers {
                out.push_str(h);
                out.push('\t');
            }
            out.push('\n');
            for row in rows {
                for cell in row {
                    out.push_str(cell);
                    out.push('\t');
                }
                out.push('\n');
            }
        }
    }
}

/// Apply inline spans to produce markdown-formatted text.
fn apply_md_spans(rt: &RichText) -> String {
    apply_spans(rt, &MdFormatter)
}

// ===========================================================================
// Lark Card renderer
// ===========================================================================

fn render_lark_card(ir: &MessageIR, options: &RenderOptions) -> String {
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

pub fn apply_lark_md_spans(rt: &RichText) -> String {
    // Lark markdown uses the same syntax as standard markdown
    apply_md_spans(rt)
}

// ===========================================================================
// Slack mrkdwn renderer
// ===========================================================================

fn render_slack(ir: &MessageIR, options: &RenderOptions) -> String {
    let mut out = String::new();
    for (i, block) in ir.blocks.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match block {
            Block::Paragraph(rt) => {
                out.push_str(&apply_spans(rt, &SlackFormatter));
                out.push('\n');
            }
            Block::CodeBlock { code, .. } => {
                out.push_str("```\n");
                out.push_str(code);
                out.push_str("\n```\n");
            }
            Block::Heading { text, .. } => {
                // Slack has no heading syntax; use bold
                out.push('*');
                out.push_str(&apply_spans(text, &SlackFormatter));
                out.push_str("*\n");
            }
            Block::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    if *ordered {
                        out.push_str(&format!("{}. ", idx + 1));
                    } else {
                        out.push_str("• ");
                    }
                    out.push_str(&apply_spans(item, &SlackFormatter));
                    out.push('\n');
                }
            }
            Block::Blockquote(rt) => {
                for line in apply_spans(rt, &SlackFormatter).lines() {
                    out.push_str("&gt; ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            Block::Table { headers, rows } => {
                render_table_plain(&mut out, headers, rows, options.table_mode);
            }
            Block::ThematicBreak => {
                out.push_str("---\n");
            }
            Block::Image { alt, url } => {
                if options.preserve_images {
                    out.push_str(&format!("<{}|{}>\n", url, alt));
                }
            }
        }
    }
    out
}

// ===========================================================================
// Telegram HTML renderer
// ===========================================================================

fn render_telegram(ir: &MessageIR, options: &RenderOptions) -> String {
    let mut out = String::new();
    for (i, block) in ir.blocks.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match block {
            Block::Paragraph(rt) => {
                out.push_str(&apply_spans(rt, &TelegramFormatter));
                out.push('\n');
            }
            Block::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    out.push_str(&format!(
                        "<pre><code class=\"language-{}\">{}</code></pre>\n",
                        escape_html(lang),
                        escape_html(code)
                    ));
                } else {
                    out.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(code)));
                }
            }
            Block::Heading { text, .. } => {
                out.push_str("<b>");
                out.push_str(&apply_spans(text, &TelegramFormatter));
                out.push_str("</b>\n");
            }
            Block::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    if *ordered {
                        out.push_str(&format!("{}. ", idx + 1));
                    } else {
                        out.push_str("• ");
                    }
                    out.push_str(&apply_spans(item, &TelegramFormatter));
                    out.push('\n');
                }
            }
            Block::Blockquote(rt) => {
                out.push_str("<blockquote>");
                out.push_str(&apply_spans(rt, &TelegramFormatter));
                out.push_str("</blockquote>\n");
            }
            Block::Table { headers, rows } => {
                render_table_plain(&mut out, headers, rows, options.table_mode);
            }
            Block::ThematicBreak => {
                out.push_str("---\n");
            }
            Block::Image { alt, url } => {
                if options.preserve_images {
                    out.push_str(&format!(
                        "<a href=\"{}\">{}</a>\n",
                        escape_html(url),
                        escape_html(alt)
                    ));
                }
            }
        }
    }
    out
}

// ===========================================================================
// Plain text renderer
// ===========================================================================

fn render_plain(ir: &MessageIR, options: &RenderOptions) -> String {
    let mut out = String::new();
    for (i, block) in ir.blocks.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match block {
            Block::Paragraph(rt) => {
                out.push_str(&apply_spans(rt, &PlainFormatter));
                out.push('\n');
            }
            Block::CodeBlock { code, .. } => {
                // Indent with 4 spaces
                for line in code.lines() {
                    out.push_str("    ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            Block::Heading { text, .. } => {
                out.push_str(&apply_spans(text, &PlainFormatter));
                out.push('\n');
            }
            Block::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    if *ordered {
                        out.push_str(&format!("{}. ", idx + 1));
                    } else {
                        out.push_str("- ");
                    }
                    out.push_str(&apply_spans(item, &PlainFormatter));
                    out.push('\n');
                }
            }
            Block::Blockquote(rt) => {
                out.push_str(&apply_spans(rt, &PlainFormatter));
                out.push('\n');
            }
            Block::Table { headers, rows } => {
                render_table_plain(&mut out, headers, rows, options.table_mode);
            }
            Block::ThematicBreak => {
                out.push_str("---\n");
            }
            Block::Image { alt, url } => {
                if options.preserve_images {
                    out.push_str(&format!("{} ({})\n", alt, url));
                }
            }
        }
    }
    out
}

fn render_table_plain(out: &mut String, headers: &[String], rows: &[Vec<String>], mode: TableMode) {
    match mode {
        TableMode::Code => {
            // Simple aligned table using padding
            let cols = headers.len();
            let mut widths = vec![0usize; cols];
            for (i, h) in headers.iter().enumerate() {
                widths[i] = widths[i].max(h.len());
            }
            for row in rows {
                for (i, cell) in row.iter().enumerate() {
                    if i < cols {
                        widths[i] = widths[i].max(cell.len());
                    }
                }
            }
            // Header
            for (i, h) in headers.iter().enumerate() {
                if i > 0 {
                    out.push_str("  ");
                }
                out.push_str(h);
                let pad = widths.get(i).copied().unwrap_or(0).saturating_sub(h.len());
                for _ in 0..pad {
                    out.push(' ');
                }
            }
            out.push('\n');
            // Separator
            for (i, w) in widths.iter().enumerate() {
                if i > 0 {
                    out.push_str("  ");
                }
                for _ in 0..*w {
                    out.push('-');
                }
            }
            out.push('\n');
            // Rows
            for row in rows {
                for (i, cell) in row.iter().enumerate() {
                    if i > 0 {
                        out.push_str("  ");
                    }
                    out.push_str(cell);
                    let pad = widths
                        .get(i)
                        .copied()
                        .unwrap_or(0)
                        .saturating_sub(cell.len());
                    for _ in 0..pad {
                        out.push(' ');
                    }
                }
                out.push('\n');
            }
        }
        TableMode::Bullets => {
            for row in rows {
                out.push_str("- ");
                let pairs: Vec<String> = headers
                    .iter()
                    .zip(row.iter())
                    .map(|(h, c)| format!("{}: {}", h, c))
                    .collect();
                out.push_str(&pairs.join(", "));
                out.push('\n');
            }
        }
        TableMode::Off => {
            for h in headers {
                out.push_str(h);
                out.push('\t');
            }
            out.push('\n');
            for row in rows {
                for cell in row {
                    out.push_str(cell);
                    out.push('\t');
                }
                out.push('\n');
            }
        }
    }
}

// ===========================================================================
// Span application framework
// ===========================================================================

/// Trait for platform-specific inline formatting.
trait InlineFormatter {
    fn wrap_bold(&self, text: &str) -> String;
    fn wrap_italic(&self, text: &str) -> String;
    fn wrap_strikethrough(&self, text: &str) -> String;
    fn wrap_code(&self, text: &str) -> String;
    fn wrap_spoiler(&self, text: &str) -> String;
    fn wrap_link(&self, label: &str, href: &str) -> String;
    /// Called for text outside any span — override to escape characters.
    fn escape_text(&self, text: &str) -> String {
        text.to_string()
    }
}

/// Apply inline spans to rich text using a given formatter.
fn apply_spans(rt: &RichText, fmt: &dyn InlineFormatter) -> String {
    if rt.styles.is_empty() && rt.links.is_empty() {
        return fmt.escape_text(&rt.text);
    }

    // Collect all span boundaries as events
    let mut events: Vec<SpanEvent> = Vec::new();

    for span in &rt.styles {
        events.push(SpanEvent {
            pos: span.start,
            is_start: true,
            priority: 0,
        });
        events.push(SpanEvent {
            pos: span.end,
            is_start: false,
            priority: 0,
        });
    }

    for link in &rt.links {
        events.push(SpanEvent {
            pos: link.start,
            is_start: true,
            priority: 1,
        });
        events.push(SpanEvent {
            pos: link.end,
            is_start: false,
            priority: 1,
        });
    }

    // Sort: by position, then ends before starts at same position
    events.sort_by(|a, b| {
        a.pos
            .cmp(&b.pos)
            .then(a.is_start.cmp(&b.is_start))
            .then(a.priority.cmp(&b.priority))
    });

    // Simpler approach: just apply wrapping per-span using byte ranges.
    // For non-overlapping spans this is straightforward.
    // For simplicity with potentially overlapping spans, use a character-by-character approach.

    let text = &rt.text;
    let mut result = String::new();
    let mut pos = 0;

    // Collect unique boundary positions
    let mut boundaries: Vec<usize> = events.iter().map(|e| e.pos).collect();
    boundaries.push(text.len());
    boundaries.sort();
    boundaries.dedup();

    for &boundary in &boundaries {
        if boundary > pos {
            // Emit the segment [pos..boundary]
            let segment = &text[pos..boundary];

            // Determine which styles/links are active at this segment
            let mut active_styles: Vec<Style> = Vec::new();
            let mut active_link: Option<&str> = None;

            for span in &rt.styles {
                if span.start <= pos && span.end >= boundary {
                    active_styles.push(span.style);
                }
            }
            for link in &rt.links {
                if link.start <= pos && link.end >= boundary {
                    active_link = Some(&link.href);
                }
            }

            let mut formatted = fmt.escape_text(segment);

            // Apply styles (innermost first)
            for style in &active_styles {
                formatted = match style {
                    Style::Bold => fmt.wrap_bold(&formatted),
                    Style::Italic => fmt.wrap_italic(&formatted),
                    Style::Strikethrough => fmt.wrap_strikethrough(&formatted),
                    Style::Code => fmt.wrap_code(&formatted),
                    Style::Spoiler => fmt.wrap_spoiler(&formatted),
                };
            }

            // Apply link (outermost)
            if let Some(href) = active_link {
                formatted = fmt.wrap_link(&formatted, href);
            }

            result.push_str(&formatted);
            pos = boundary;
        }
    }

    // Emit remaining text
    if pos < text.len() {
        result.push_str(&fmt.escape_text(&text[pos..]));
    }

    result
}

struct SpanEvent {
    pos: usize,
    is_start: bool,
    priority: u8,
}

// ===========================================================================
// Per-platform formatters
// ===========================================================================

struct MdFormatter;

impl InlineFormatter for MdFormatter {
    fn wrap_bold(&self, text: &str) -> String {
        format!("**{}**", text)
    }
    fn wrap_italic(&self, text: &str) -> String {
        format!("*{}*", text)
    }
    fn wrap_strikethrough(&self, text: &str) -> String {
        format!("~~{}~~", text)
    }
    fn wrap_code(&self, text: &str) -> String {
        format!("`{}`", text)
    }
    fn wrap_spoiler(&self, text: &str) -> String {
        format!("||{}||", text)
    }
    fn wrap_link(&self, label: &str, href: &str) -> String {
        format!("[{}]({})", label, href)
    }
}

struct SlackFormatter;

impl InlineFormatter for SlackFormatter {
    fn wrap_bold(&self, text: &str) -> String {
        format!("*{}*", text)
    }
    fn wrap_italic(&self, text: &str) -> String {
        format!("_{}_", text)
    }
    fn wrap_strikethrough(&self, text: &str) -> String {
        format!("~{}~", text)
    }
    fn wrap_code(&self, text: &str) -> String {
        format!("`{}`", text)
    }
    fn wrap_spoiler(&self, text: &str) -> String {
        // Slack doesn't have spoiler syntax
        text.to_string()
    }
    fn wrap_link(&self, label: &str, href: &str) -> String {
        format!("<{}|{}>", href, label)
    }
}

struct TelegramFormatter;

impl InlineFormatter for TelegramFormatter {
    fn wrap_bold(&self, text: &str) -> String {
        format!("<b>{}</b>", text)
    }
    fn wrap_italic(&self, text: &str) -> String {
        format!("<i>{}</i>", text)
    }
    fn wrap_strikethrough(&self, text: &str) -> String {
        format!("<s>{}</s>", text)
    }
    fn wrap_code(&self, text: &str) -> String {
        format!("<code>{}</code>", text)
    }
    fn wrap_spoiler(&self, text: &str) -> String {
        format!("<tg-spoiler>{}</tg-spoiler>", text)
    }
    fn wrap_link(&self, label: &str, href: &str) -> String {
        format!("<a href=\"{}\">{}</a>", escape_html(href), label)
    }
    fn escape_text(&self, text: &str) -> String {
        escape_html(text)
    }
}

struct PlainFormatter;

impl InlineFormatter for PlainFormatter {
    fn wrap_bold(&self, text: &str) -> String {
        text.to_string()
    }
    fn wrap_italic(&self, text: &str) -> String {
        text.to_string()
    }
    fn wrap_strikethrough(&self, text: &str) -> String {
        text.to_string()
    }
    fn wrap_code(&self, text: &str) -> String {
        text.to_string()
    }
    fn wrap_spoiler(&self, text: &str) -> String {
        text.to_string()
    }
    fn wrap_link(&self, label: &str, href: &str) -> String {
        format!("{} ({})", label, href)
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ir() -> MessageIR {
        MessageIR {
            blocks: vec![
                Block::Heading {
                    level: 1,
                    text: RichText::plain("Title"),
                },
                Block::Paragraph(RichText {
                    text: "Hello bold world".to_string(),
                    styles: vec![StyleSpan {
                        start: 6,
                        end: 10,
                        style: Style::Bold,
                    }],
                    links: vec![],
                }),
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "fn main() {}".into(),
                },
                Block::List {
                    ordered: false,
                    items: vec![RichText::plain("first"), RichText::plain("second")],
                },
                Block::Blockquote(RichText::plain("a quote")),
                Block::Table {
                    headers: vec!["A".into(), "B".into()],
                    rows: vec![vec!["1".into(), "2".into()]],
                },
                Block::ThematicBreak,
                Block::Image {
                    alt: "logo".into(),
                    url: "https://img.png".into(),
                },
            ],
        }
    }

    #[test]
    fn render_markdown_output() {
        let ir = sample_ir();
        let opts = RenderOptions::new(RenderTarget::Markdown);
        let out = render(&ir, &opts);
        assert!(out.contains("# Title"), "heading: {}", out);
        assert!(out.contains("**bold**"), "bold: {}", out);
        assert!(out.contains("```rust"), "code fence: {}", out);
        assert!(out.contains("- first"), "list: {}", out);
        assert!(out.contains("> a quote"), "blockquote: {}", out);
        assert!(out.contains("| A |"), "table: {}", out);
        assert!(out.contains("---"), "hr: {}", out);
        assert!(out.contains("![logo](https://img.png)"), "image: {}", out);
    }

    #[test]
    fn render_slack_output() {
        let ir = sample_ir();
        let opts = RenderOptions::new(RenderTarget::SlackMrkdwn);
        let out = render(&ir, &opts);
        assert!(out.contains("*bold*"), "bold: {}", out);
        assert!(out.contains("```"), "code: {}", out);
        assert!(out.contains("• first"), "list: {}", out);
    }

    #[test]
    fn render_telegram_output() {
        let ir = sample_ir();
        let opts = RenderOptions::new(RenderTarget::TelegramHtml);
        let out = render(&ir, &opts);
        assert!(out.contains("<b>bold</b>"), "bold: {}", out);
        assert!(
            out.contains("<pre><code class=\"language-rust\">"),
            "code: {}",
            out
        );
    }

    #[test]
    fn render_plain_output() {
        let ir = sample_ir();
        let opts = RenderOptions::new(RenderTarget::PlainText);
        let out = render(&ir, &opts);
        // No markdown syntax
        assert!(!out.contains("**"), "no bold markers: {}", out);
        assert!(out.contains("Hello bold world"), "text: {}", out);
        assert!(out.contains("    fn main() {}"), "indented code: {}", out);
    }

    #[test]
    fn render_lark_card_output() {
        let ir = sample_ir();
        let opts = RenderOptions::new(RenderTarget::LarkCard);
        let out = render(&ir, &opts);
        assert!(out.starts_with('['), "json array: {}", out);
        assert!(out.ends_with(']'), "json array end: {}", out);
        assert!(out.contains("\"tag\":\"div\""), "div element: {}", out);
        assert!(out.contains("\"tag\":\"hr\""), "hr element: {}", out);
    }

    #[test]
    fn render_link_markdown() {
        let ir = MessageIR {
            blocks: vec![Block::Paragraph(RichText {
                text: "click here".to_string(),
                styles: vec![],
                links: vec![LinkSpan {
                    start: 6,
                    end: 10,
                    href: "https://example.com".into(),
                }],
            })],
        };
        let opts = RenderOptions::new(RenderTarget::Markdown);
        let out = render(&ir, &opts);
        assert!(out.contains("[here](https://example.com)"), "link: {}", out);
    }

    #[test]
    fn render_link_slack() {
        let ir = MessageIR {
            blocks: vec![Block::Paragraph(RichText {
                text: "click here".to_string(),
                styles: vec![],
                links: vec![LinkSpan {
                    start: 6,
                    end: 10,
                    href: "https://example.com".into(),
                }],
            })],
        };
        let opts = RenderOptions::new(RenderTarget::SlackMrkdwn);
        let out = render(&ir, &opts);
        assert!(out.contains("<https://example.com|here>"), "link: {}", out);
    }

    #[test]
    fn render_link_telegram() {
        let ir = MessageIR {
            blocks: vec![Block::Paragraph(RichText {
                text: "click here".to_string(),
                styles: vec![],
                links: vec![LinkSpan {
                    start: 6,
                    end: 10,
                    href: "https://example.com".into(),
                }],
            })],
        };
        let opts = RenderOptions::new(RenderTarget::TelegramHtml);
        let out = render(&ir, &opts);
        assert!(
            out.contains("<a href=\"https://example.com\">here</a>"),
            "link: {}",
            out
        );
    }

    #[test]
    fn render_link_plain() {
        let ir = MessageIR {
            blocks: vec![Block::Paragraph(RichText {
                text: "click here".to_string(),
                styles: vec![],
                links: vec![LinkSpan {
                    start: 6,
                    end: 10,
                    href: "https://example.com".into(),
                }],
            })],
        };
        let opts = RenderOptions::new(RenderTarget::PlainText);
        let out = render(&ir, &opts);
        assert!(out.contains("here (https://example.com)"), "link: {}", out);
    }

    #[test]
    fn telegram_escapes_html() {
        let ir = MessageIR {
            blocks: vec![Block::Paragraph(RichText::plain("a < b & c > d"))],
        };
        let opts = RenderOptions::new(RenderTarget::TelegramHtml);
        let out = render(&ir, &opts);
        assert!(out.contains("a &lt; b &amp; c &gt; d"), "escaped: {}", out);
    }

    #[test]
    fn table_bullets_mode() {
        let ir = MessageIR {
            blocks: vec![Block::Table {
                headers: vec!["Name".into(), "Age".into()],
                rows: vec![vec!["Alice".into(), "30".into()]],
            }],
        };
        let opts = RenderOptions::new(RenderTarget::PlainText).with_table_mode(TableMode::Bullets);
        let out = render(&ir, &opts);
        assert!(out.contains("- Name: Alice, Age: 30"), "bullets: {}", out);
    }
}
