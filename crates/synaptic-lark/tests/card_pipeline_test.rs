use synaptic_core::message_ir::{parse_markdown, RenderOptions, RenderTarget};
use synaptic_lark::card_elements::render_lark_card_elements;

#[test]
fn test_full_pipeline_produces_valid_elements() {
    let markdown = r#"# Analysis Report

Here is the **summary** of findings:

1. First item
2. Second item

```python
def analyze():
    return "done"
```

---

> Note: This is important.

![diagram](img_v3_abc123)
"#;

    let ir = parse_markdown(markdown);
    let options = RenderOptions::new(RenderTarget::LarkCard);
    let elements = render_lark_card_elements(&ir, &options);

    // Verify element count and types
    assert!(
        elements.len() >= 6,
        "expected heading, paragraph, list, code, hr, blockquote, image but got {}",
        elements.len()
    );

    // Verify all element_ids are unique
    let ids: Vec<&str> = elements.iter().map(|e| e.element_id.as_str()).collect();
    let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique.len(), "element IDs must be unique");

    // Verify element_ids follow constraints (alphanumeric + underscore, starts with letter, ≤20 chars)
    for elem in &elements {
        assert!(
            elem.element_id.len() <= 20,
            "element_id too long: {}",
            elem.element_id
        );
        assert!(
            elem.element_id
                .chars()
                .next()
                .unwrap()
                .is_ascii_alphabetic(),
            "element_id must start with letter: {}",
            elem.element_id
        );
    }

    // Verify image with img_key renders as native img
    let img = elements.iter().find(|e| e.tag == "img");
    assert!(
        img.is_some(),
        "img_v3_ URL should produce native img element"
    );
    assert_eq!(img.unwrap().properties["img_key"], "img_v3_abc123");

    // Verify heading has text_size
    assert_eq!(elements[0].tag, "markdown");
    assert_eq!(elements[0].properties["text_size"], "heading-1");

    // Verify HR element
    let hr = elements.iter().find(|e| e.tag == "hr");
    assert!(hr.is_some(), "should have hr element for thematic break");
}

#[test]
fn test_empty_markdown_produces_no_elements() {
    let ir = parse_markdown("");
    let options = RenderOptions::new(RenderTarget::LarkCard);
    let elements = render_lark_card_elements(&ir, &options);
    assert!(elements.is_empty());
}

#[test]
fn test_elements_serialize_to_valid_json() {
    let ir = parse_markdown("# Title\n\nHello **world**");
    let options = RenderOptions::new(RenderTarget::LarkCard);
    let elements = render_lark_card_elements(&ir, &options);

    // Each element should serialize to valid JSON
    for elem in &elements {
        let json = serde_json::to_value(elem).unwrap();
        assert!(json.get("tag").is_some());
        assert!(json.get("element_id").is_some());
    }
}
