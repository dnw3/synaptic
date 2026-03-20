use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::*;

/// Parse a Markdown string into a [`MessageIR`].
pub fn parse(markdown: &str) -> MessageIR {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut builder = IrBuilder::new();

    for event in parser {
        builder.process(event);
    }

    builder.finish()
}

// ---------------------------------------------------------------------------
// Builder state machine
// ---------------------------------------------------------------------------

struct IrBuilder {
    blocks: Vec<Block>,

    // Current rich-text being accumulated
    current_text: RichText,
    style_stack: Vec<Style>,

    // Active link destination (set between Start(Link) and End(Link))
    active_link: Option<String>,
    link_start: usize,

    // Code block accumulation
    code_lang: Option<String>,
    code_buf: String,
    in_code_block: bool,

    // Heading
    heading_level: u8,
    in_heading: bool,

    // Table
    in_table: bool,
    in_table_head: bool,
    table_headers: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_row: Vec<String>,
    table_cell: String,

    // List
    in_list: bool,
    list_ordered: bool,
    list_items: Vec<RichText>,
    in_item: bool,

    // Blockquote
    in_blockquote: bool,
    blockquote_text: RichText,

    // Image (we need to capture alt text between Start(Image) and End(Image))
    in_image: bool,
    image_url: String,
    image_alt: String,
}

impl IrBuilder {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_text: RichText::default(),
            style_stack: Vec::new(),
            active_link: None,
            link_start: 0,
            code_lang: None,
            code_buf: String::new(),
            in_code_block: false,
            heading_level: 0,
            in_heading: false,
            in_table: false,
            in_table_head: false,
            table_headers: Vec::new(),
            table_rows: Vec::new(),
            table_row: Vec::new(),
            table_cell: String::new(),
            in_list: false,
            list_ordered: false,
            list_items: Vec::new(),
            in_item: false,
            in_blockquote: false,
            blockquote_text: RichText::default(),
            in_image: false,
            image_url: String::new(),
            image_alt: String::new(),
        }
    }

    fn process(&mut self, event: Event) {
        match event {
            // ---- Block-level starts ----
            Event::Start(Tag::Paragraph) => {
                // If inside a blockquote or list item, just keep accumulating
            }

            Event::Start(Tag::Heading { level, .. }) => {
                self.in_heading = true;
                self.heading_level = heading_level_to_u8(level);
                self.current_text = RichText::default();
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                self.in_code_block = true;
                self.code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang = lang.to_string();
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang)
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
                self.code_buf.clear();
            }

            Event::Start(Tag::BlockQuote(_)) => {
                self.in_blockquote = true;
                self.blockquote_text = RichText::default();
            }

            Event::Start(Tag::List(start)) => {
                self.in_list = true;
                self.list_ordered = start.is_some();
                self.list_items.clear();
            }

            Event::Start(Tag::Item) => {
                self.in_item = true;
                self.current_text = RichText::default();
            }

            Event::Start(Tag::Table(_)) => {
                self.in_table = true;
                self.table_headers.clear();
                self.table_rows.clear();
            }

            Event::Start(Tag::TableHead) => {
                self.in_table_head = true;
                self.table_row.clear();
            }

            Event::Start(Tag::TableRow) => {
                self.table_row.clear();
            }

            Event::Start(Tag::TableCell) => {
                self.table_cell.clear();
            }

            // ---- Inline style starts ----
            Event::Start(Tag::Emphasis) => {
                self.style_stack.push(Style::Italic);
            }

            Event::Start(Tag::Strong) => {
                self.style_stack.push(Style::Bold);
            }

            Event::Start(Tag::Strikethrough) => {
                self.style_stack.push(Style::Strikethrough);
            }

            Event::Start(Tag::Link { dest_url, .. }) => {
                let target = self.active_rich_text();
                self.link_start = target.text.len();
                self.active_link = Some(dest_url.to_string());
            }

            Event::Start(Tag::Image { dest_url, .. }) => {
                self.in_image = true;
                self.image_url = dest_url.to_string();
                self.image_alt.clear();
            }

            // ---- Text content ----
            Event::Text(text) => {
                if self.in_image {
                    self.image_alt.push_str(&text);
                    return;
                }
                if self.in_code_block {
                    self.code_buf.push_str(&text);
                    return;
                }
                if self.in_table {
                    self.table_cell.push_str(&text);
                    return;
                }
                self.append_text(&text);
            }

            Event::Code(text) => {
                if self.in_table {
                    self.table_cell.push_str(&text);
                    return;
                }
                let target = self.active_rich_text_mut();
                let start = target.text.len();
                target.text.push_str(&text);
                let end = target.text.len();
                target.styles.push(StyleSpan {
                    start,
                    end,
                    style: Style::Code,
                });
            }

            Event::SoftBreak | Event::HardBreak => {
                if self.in_code_block {
                    self.code_buf.push('\n');
                    return;
                }
                if self.in_table {
                    self.table_cell.push(' ');
                    return;
                }
                let target = self.active_rich_text_mut();
                target.text.push('\n');
            }

            Event::Rule => {
                self.blocks.push(Block::ThematicBreak);
            }

            // ---- Block-level ends ----
            Event::End(TagEnd::Paragraph) => {
                if self.in_blockquote || self.in_item || self.in_table {
                    // Nested paragraph inside blockquote/list/table — don't emit block
                    return;
                }
                let text = std::mem::take(&mut self.current_text);
                if !text.text.is_empty() {
                    self.blocks.push(Block::Paragraph(text));
                }
            }

            Event::End(TagEnd::Heading(_)) => {
                self.in_heading = false;
                let text = std::mem::take(&mut self.current_text);
                self.blocks.push(Block::Heading {
                    level: self.heading_level,
                    text,
                });
            }

            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
                // Remove trailing newline if present
                if self.code_buf.ends_with('\n') {
                    self.code_buf.pop();
                }
                self.blocks.push(Block::CodeBlock {
                    language: self.code_lang.take(),
                    code: std::mem::take(&mut self.code_buf),
                });
            }

            Event::End(TagEnd::BlockQuote(_)) => {
                self.in_blockquote = false;
                let text = std::mem::take(&mut self.blockquote_text);
                if !text.text.is_empty() {
                    self.blocks.push(Block::Blockquote(text));
                }
            }

            Event::End(TagEnd::Item) => {
                self.in_item = false;
                let text = std::mem::take(&mut self.current_text);
                self.list_items.push(text);
            }

            Event::End(TagEnd::List(_)) => {
                self.in_list = false;
                let items = std::mem::take(&mut self.list_items);
                self.blocks.push(Block::List {
                    ordered: self.list_ordered,
                    items,
                });
            }

            Event::End(TagEnd::TableCell) => {
                self.table_row.push(std::mem::take(&mut self.table_cell));
            }

            Event::End(TagEnd::TableHead) => {
                self.in_table_head = false;
                self.table_headers = std::mem::take(&mut self.table_row);
            }

            Event::End(TagEnd::TableRow) => {
                let row = std::mem::take(&mut self.table_row);
                self.table_rows.push(row);
            }

            Event::End(TagEnd::Table) => {
                self.in_table = false;
                self.blocks.push(Block::Table {
                    headers: std::mem::take(&mut self.table_headers),
                    rows: std::mem::take(&mut self.table_rows),
                });
            }

            // ---- Inline style ends ----
            Event::End(TagEnd::Emphasis) => {
                self.style_stack.retain(|s| *s != Style::Italic);
            }

            Event::End(TagEnd::Strong) => {
                self.style_stack.retain(|s| *s != Style::Bold);
            }

            Event::End(TagEnd::Strikethrough) => {
                self.style_stack.retain(|s| *s != Style::Strikethrough);
            }

            Event::End(TagEnd::Link) => {
                if let Some(href) = self.active_link.take() {
                    let link_start = self.link_start;
                    let target = self.active_rich_text_mut();
                    let end = target.text.len();
                    if link_start < end {
                        target.links.push(LinkSpan {
                            start: link_start,
                            end,
                            href,
                        });
                    }
                }
            }

            Event::End(TagEnd::Image) => {
                self.in_image = false;
                self.blocks.push(Block::Image {
                    alt: std::mem::take(&mut self.image_alt),
                    url: std::mem::take(&mut self.image_url),
                });
            }

            // Ignore other events
            _ => {}
        }
    }

    /// Get a mutable reference to the currently-active RichText target.
    fn active_rich_text_mut(&mut self) -> &mut RichText {
        if self.in_blockquote && !self.in_item {
            &mut self.blockquote_text
        } else {
            &mut self.current_text
        }
    }

    /// Get a reference to the currently-active RichText target.
    fn active_rich_text(&self) -> &RichText {
        if self.in_blockquote && !self.in_item {
            &self.blockquote_text
        } else {
            &self.current_text
        }
    }

    /// Append text to the active RichText, recording any active style spans.
    fn append_text(&mut self, text: &str) {
        // Collect styles first to avoid borrow conflict
        let styles: Vec<Style> = self.style_stack.clone();
        let target = self.active_rich_text_mut();
        let start = target.text.len();
        target.text.push_str(text);
        let end = target.text.len();

        for style in styles {
            target.styles.push(StyleSpan { start, end, style });
        }
    }

    fn finish(mut self) -> MessageIR {
        // Flush any remaining text
        let text = std::mem::take(&mut self.current_text);
        if !text.text.is_empty() {
            self.blocks.push(Block::Paragraph(text));
        }
        MessageIR {
            blocks: self.blocks,
        }
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_paragraph() {
        let ir = parse("Hello, world!");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Paragraph(rt) => assert_eq!(rt.text, "Hello, world!"),
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn parse_heading() {
        let ir = parse("## My Heading");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Heading { level, text } => {
                assert_eq!(*level, 2);
                assert_eq!(text.text, "My Heading");
            }
            other => panic!("expected Heading, got {:?}", other),
        }
    }

    #[test]
    fn parse_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let ir = parse(md);
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::CodeBlock { language, code } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(code, "fn main() {}");
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn parse_bold_italic() {
        let ir = parse("Hello **bold** and *italic* text");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Paragraph(rt) => {
                assert_eq!(rt.text, "Hello bold and italic text");
                // Check bold span
                let bold = rt.styles.iter().find(|s| s.style == Style::Bold).unwrap();
                assert_eq!(&rt.text[bold.start..bold.end], "bold");
                // Check italic span
                let italic = rt.styles.iter().find(|s| s.style == Style::Italic).unwrap();
                assert_eq!(&rt.text[italic.start..italic.end], "italic");
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn parse_inline_code() {
        let ir = parse("Use `println!` here");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Paragraph(rt) => {
                assert_eq!(rt.text, "Use println! here");
                let code = rt.styles.iter().find(|s| s.style == Style::Code).unwrap();
                assert_eq!(&rt.text[code.start..code.end], "println!");
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn parse_link() {
        let ir = parse("Click [here](https://example.com) now");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Paragraph(rt) => {
                assert_eq!(rt.text, "Click here now");
                assert_eq!(rt.links.len(), 1);
                assert_eq!(&rt.text[rt.links[0].start..rt.links[0].end], "here");
                assert_eq!(rt.links[0].href, "https://example.com");
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn parse_list() {
        let md = "- first\n- second\n- third";
        let ir = parse(md);
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::List { ordered, items } => {
                assert!(!ordered);
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].text, "first");
                assert_eq!(items[1].text, "second");
                assert_eq!(items[2].text, "third");
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn parse_ordered_list() {
        let md = "1. alpha\n2. beta";
        let ir = parse(md);
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::List { ordered, items } => {
                assert!(ordered);
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn parse_blockquote() {
        let ir = parse("> quoted text");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Blockquote(rt) => {
                assert_eq!(rt.text, "quoted text");
            }
            other => panic!("expected Blockquote, got {:?}", other),
        }
    }

    #[test]
    fn parse_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let ir = parse(md);
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Table { headers, rows } => {
                assert_eq!(headers, &["A", "B"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], vec!["1", "2"]);
                assert_eq!(rows[1], vec!["3", "4"]);
            }
            other => panic!("expected Table, got {:?}", other),
        }
    }

    #[test]
    fn parse_thematic_break() {
        let md = "before\n\n---\n\nafter";
        let ir = parse(md);
        assert!(ir.blocks.iter().any(|b| matches!(b, Block::ThematicBreak)));
    }

    #[test]
    fn parse_image() {
        let ir = parse("![alt text](https://img.png)");
        assert_eq!(ir.blocks.len(), 1);
        match &ir.blocks[0] {
            Block::Image { alt, url } => {
                assert_eq!(alt, "alt text");
                assert_eq!(url, "https://img.png");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    #[test]
    fn parse_complex_document() {
        let md = r#"# Title

Some **bold** and *italic* text with `code`.

```python
print("hello")
```

- item one
- item two

> a quote

| Col1 | Col2 |
|------|------|
| a    | b    |

---

![logo](https://logo.png)
"#;
        let ir = parse(md);
        // Should have: Heading, Paragraph, CodeBlock, List, Blockquote, Table, ThematicBreak, Image
        assert!(ir.blocks.len() >= 7, "got {} blocks", ir.blocks.len());
    }
}
