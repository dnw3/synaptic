use synaptic_core::message_ir::{
    parse_markdown, Block, MessageIR, RenderOptions, RenderTarget, RichText,
};
use synaptic_lark::card_elements::render_lark_card_elements;

#[test]
fn test_heading_renders_as_markdown_element_with_text_size() {
    let ir = parse_markdown("# Hello World");
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    assert_eq!(elems.len(), 1);
    assert_eq!(elems[0].tag, "markdown");
    assert_eq!(elems[0].properties["text_size"], "heading-1");
    assert_eq!(elems[0].properties["content"], "Hello World");
}

#[test]
fn test_paragraph_renders_as_markdown_element() {
    let ir = parse_markdown("Hello **bold** and *italic* world");
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    assert_eq!(elems.len(), 1);
    assert_eq!(elems[0].tag, "markdown");
    let content = elems[0].properties["content"].as_str().unwrap();
    assert!(content.contains("**bold**"), "bold preserved: {}", content);
    assert!(
        content.contains("*italic*"),
        "italic preserved: {}",
        content
    );
}

#[test]
fn test_code_block_renders_as_markdown_element() {
    let ir = parse_markdown("```rust\nfn main() {}\n```");
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    assert_eq!(elems.len(), 1);
    assert_eq!(elems[0].tag, "markdown");
    let content = elems[0].properties["content"].as_str().unwrap();
    assert!(content.contains("```rust"), "has lang: {}", content);
    assert!(content.contains("fn main() {}"), "has code: {}", content);
}

#[test]
fn test_thematic_break_renders_as_hr() {
    let ir = parse_markdown("---");
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    assert_eq!(elems.len(), 1);
    assert_eq!(elems[0].tag, "hr");
}

#[test]
fn test_image_renders_as_markdown_fallback() {
    let ir = MessageIR {
        blocks: vec![Block::Image {
            alt: "logo".into(),
            url: "https://example.com/logo.png".into(),
        }],
    };
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    assert_eq!(elems.len(), 1);
    assert_eq!(elems[0].tag, "markdown");
    let content = elems[0].properties["content"].as_str().unwrap();
    assert!(
        content.contains("![logo](https://example.com/logo.png)"),
        "markdown image: {}",
        content
    );
}

#[test]
fn test_image_with_img_key_renders_as_native_img() {
    let ir = MessageIR {
        blocks: vec![Block::Image {
            alt: "photo".into(),
            url: "img_v2_abcdef".into(),
        }],
    };
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    assert_eq!(elems.len(), 1);
    assert_eq!(elems[0].tag, "img");
    assert_eq!(elems[0].properties["img_key"], "img_v2_abcdef");
}

#[test]
fn test_element_ids_are_unique() {
    let ir = parse_markdown("# Title\n\nParagraph\n\n---\n\n## Another");
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);
    let ids: Vec<&str> = elems.iter().map(|e| e.element_id.as_str()).collect();
    let mut deduped = ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(ids.len(), deduped.len(), "IDs must be unique: {:?}", ids);
}

#[test]
fn test_heading_levels_map_to_text_size() {
    let cases = vec![
        (1, "heading-1"),
        (2, "heading-2"),
        (3, "heading-3"),
        (4, "heading-4"),
        (5, "normal"),
        (6, "normal"),
    ];
    for (level, expected_size) in cases {
        let ir = MessageIR {
            blocks: vec![Block::Heading {
                level,
                text: RichText::plain("Test"),
            }],
        };
        let opts = RenderOptions::new(RenderTarget::LarkCard);
        let elems = render_lark_card_elements(&ir, &opts);
        assert_eq!(
            elems[0].properties["text_size"], expected_size,
            "level {} should map to {}",
            level, expected_size
        );
    }
}

#[test]
fn test_complex_document() {
    let md = "# Title\n\nSome **bold** text.\n\n```python\nprint('hi')\n```\n\n---\n\n- item1\n- item2\n";
    let ir = parse_markdown(md);
    let opts = RenderOptions::new(RenderTarget::LarkCard);
    let elems = render_lark_card_elements(&ir, &opts);

    // Should have: heading, paragraph, code, hr, list = 5 elements
    assert!(
        elems.len() >= 5,
        "expected >= 5 elements, got {}",
        elems.len()
    );

    // Verify types
    assert_eq!(elems[0].tag, "markdown"); // heading
    assert_eq!(elems[0].properties["text_size"], "heading-1");
    assert_eq!(elems[1].tag, "markdown"); // paragraph
    assert_eq!(elems[2].tag, "markdown"); // code block

    // Find hr
    let hr = elems.iter().find(|e| e.tag == "hr");
    assert!(hr.is_some(), "should have an hr element");

    // Find list
    let list_elem = elems.iter().find(|e| {
        e.tag == "markdown"
            && e.properties["content"]
                .as_str()
                .map_or(false, |c| c.contains("- item1"))
    });
    assert!(list_elem.is_some(), "should have a list element");
}
