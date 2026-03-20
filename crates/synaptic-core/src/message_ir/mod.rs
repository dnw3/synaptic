//! Message IR (Intermediate Representation) for platform-adaptive formatting.
//!
//! Parse Markdown once into a structured IR, then render per-platform:
//! - Markdown (Discord, Mattermost)
//! - Lark Card JSON (rich text elements)
//! - Slack mrkdwn
//! - Telegram HTML
//! - Signal text + UTF-16 style ranges
//! - Plain text (WhatsApp, iMessage, SMS)

#[cfg(feature = "message-ir")]
mod chunker;
#[cfg(feature = "message-ir")]
mod parser;
#[cfg(feature = "message-ir")]
mod renderer;
#[cfg(feature = "message-ir")]
pub use renderer::{
    apply_lark_md_spans, apply_spans, escape_html, escape_json_string, format_with_renderer,
    render_table_md, render_table_plain, IRRenderer, InlineFormatter, MarkdownRenderer,
    PlainTextRenderer,
};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core IR types
// ---------------------------------------------------------------------------

/// Structured intermediate representation of a formatted message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageIR {
    pub blocks: Vec<Block>,
}

/// A content block in the IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    /// Rich text paragraph.
    Paragraph(RichText),
    /// Fenced or indented code block.
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    /// Heading (level 1-6).
    Heading { level: u8, text: RichText },
    /// Ordered or unordered list.
    List { ordered: bool, items: Vec<RichText> },
    /// Block quote.
    Blockquote(RichText),
    /// Table with headers and rows.
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// Horizontal rule.
    ThematicBreak,
    /// Image with alt text and URL.
    Image { alt: String, url: String },
}

/// Rich text with inline formatting spans.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RichText {
    /// Plain text content (all markdown syntax removed).
    pub text: String,
    /// Inline style spans (byte offsets into `text`).
    pub styles: Vec<StyleSpan>,
    /// Link spans (byte offsets into `text`).
    pub links: Vec<LinkSpan>,
}

/// An inline style span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSpan {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
    /// Style type.
    pub style: Style,
}

/// Inline style types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    Bold,
    Italic,
    Strikethrough,
    Code,
    Spoiler,
}

/// A link span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkSpan {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
    /// Link target URL.
    pub href: String,
}

// ---------------------------------------------------------------------------
// Render configuration
// ---------------------------------------------------------------------------

/// Target platform for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    /// Standard Markdown (Discord, Mattermost).
    Markdown,
    /// Lark interactive card JSON (rich text elements).
    LarkCard,
    /// Lark plain text fallback.
    LarkText,
    /// Slack mrkdwn format.
    SlackMrkdwn,
    /// Telegram HTML parse mode.
    TelegramHtml,
    /// Signal: plain text + UTF-16 style ranges.
    SignalRanges,
    /// Plain text with all formatting stripped.
    PlainText,
}

/// Table rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableMode {
    /// Render as fenced code block (default).
    #[default]
    Code,
    /// Convert rows to bullet lists.
    Bullets,
    /// Pass raw table text through.
    Off,
}

/// Options for rendering IR to a target format.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub target: RenderTarget,
    pub table_mode: TableMode,
    pub chunk_limit: usize,
    pub preserve_images: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self::new(RenderTarget::PlainText)
    }
}

impl RenderOptions {
    /// Create options with default target (PlainText) and default chunk limit.
    /// For trait-based rendering, the renderer determines the format; target is ignored.
    pub fn new_default() -> Self {
        Self::new(RenderTarget::PlainText)
    }

    /// Create options for a specific target with default chunk limit.
    pub fn new(target: RenderTarget) -> Self {
        let chunk_limit = match target {
            RenderTarget::Markdown => 2000,     // Discord
            RenderTarget::LarkCard => 30000,    // Lark card content
            RenderTarget::LarkText => 4096,     // Lark text
            RenderTarget::SlackMrkdwn => 4000,  // Slack
            RenderTarget::TelegramHtml => 4096, // Telegram
            RenderTarget::SignalRanges => 6000, // Signal
            RenderTarget::PlainText => 4096,    // Generic
        };
        let table_mode = match target {
            RenderTarget::SignalRanges | RenderTarget::PlainText => TableMode::Bullets,
            _ => TableMode::Code,
        };
        Self {
            target,
            table_mode,
            chunk_limit,
            preserve_images: true,
        }
    }

    /// Override chunk limit.
    pub fn with_chunk_limit(mut self, limit: usize) -> Self {
        self.chunk_limit = limit;
        self
    }

    /// Override table mode.
    pub fn with_table_mode(mut self, mode: TableMode) -> Self {
        self.table_mode = mode;
        self
    }
}

// ---------------------------------------------------------------------------
// Convenience API
// ---------------------------------------------------------------------------

impl MessageIR {
    /// Create an empty IR.
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Render this IR to a target format, returning chunked strings.
    #[cfg(feature = "message-ir")]
    pub fn render_chunked(&self, options: &RenderOptions) -> Vec<String> {
        let chunks = chunker::chunk_ir(self, options.chunk_limit);
        chunks
            .iter()
            .map(|c| renderer::render(c, options))
            .collect()
    }

    /// Render this IR to a single string (no chunking).
    #[cfg(feature = "message-ir")]
    pub fn render(&self, options: &RenderOptions) -> String {
        renderer::render(self, options)
    }

    /// Estimate the plain text length of this IR.
    pub fn text_len(&self) -> usize {
        self.blocks
            .iter()
            .map(|b| match b {
                Block::Paragraph(rt) | Block::Heading { text: rt, .. } | Block::Blockquote(rt) => {
                    rt.text.len()
                }
                Block::CodeBlock { code, .. } => code.len() + 8, // fences
                Block::List { items, .. } => items.iter().map(|i| i.text.len() + 4).sum(),
                Block::Table { headers, rows, .. } => {
                    headers.iter().map(|h| h.len()).sum::<usize>()
                        + rows
                            .iter()
                            .flat_map(|r| r.iter().map(|c| c.len()))
                            .sum::<usize>()
                }
                Block::ThematicBreak => 4,
                Block::Image { alt, url, .. } => alt.len() + url.len() + 5,
            })
            .sum()
    }
}

impl Default for MessageIR {
    fn default() -> Self {
        Self::new()
    }
}

impl RichText {
    /// Create plain text with no formatting.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            styles: Vec::new(),
            links: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level API
// ---------------------------------------------------------------------------

/// Parse Markdown text into a MessageIR.
#[cfg(feature = "message-ir")]
pub fn parse_markdown(markdown: &str) -> MessageIR {
    parser::parse(markdown)
}

/// Format Markdown for a specific channel, returning chunked strings ready to send.
#[cfg(feature = "message-ir")]
pub fn format_for_channel(markdown: &str, options: &RenderOptions) -> Vec<String> {
    let ir = parse_markdown(markdown);
    ir.render_chunked(options)
}
