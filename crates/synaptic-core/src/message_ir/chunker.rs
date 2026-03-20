use super::*;

/// Split an IR into chunks that each fit within `limit` bytes (estimated).
///
/// Code blocks are never split across chunks. Other blocks are moved whole
/// where possible; if a single block exceeds the limit it is placed alone
/// in its own chunk (the platform renderer will truncate if needed).
pub fn chunk_ir(ir: &MessageIR, limit: usize) -> Vec<MessageIR> {
    if ir.text_len() <= limit {
        return vec![ir.clone()];
    }

    let mut chunks = Vec::new();
    let mut current = MessageIR::new();
    let mut current_len: usize = 0;

    for block in &ir.blocks {
        let block_len = estimate_block_len(block);

        // Code blocks: never split — flush current chunk if adding would overflow
        if matches!(block, Block::CodeBlock { .. }) {
            if current_len > 0 && current_len + block_len > limit {
                chunks.push(std::mem::take(&mut current));
                current_len = 0;
            }
            current.blocks.push(block.clone());
            current_len += block_len;
            continue;
        }

        // Other blocks: keep together when possible
        if current_len + block_len <= limit {
            current.blocks.push(block.clone());
            current_len += block_len;
        } else {
            if current_len > 0 {
                chunks.push(std::mem::take(&mut current));
                current_len = 0;
            }
            // If a single block exceeds limit, add it as-is
            current.blocks.push(block.clone());
            current_len += block_len;
        }
    }

    if !current.blocks.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(ir.clone());
    }

    chunks
}

/// Estimate the rendered length of a single block in bytes.
fn estimate_block_len(block: &Block) -> usize {
    match block {
        Block::Paragraph(rt) | Block::Heading { text: rt, .. } | Block::Blockquote(rt) => {
            rt.text.len() + 4 // small overhead for markers
        }
        Block::CodeBlock { code, language, .. } => {
            // ```lang\ncode\n```
            code.len() + language.as_ref().map_or(0, |l| l.len()) + 8
        }
        Block::List { items, .. } => items.iter().map(|i| i.text.len() + 4).sum(),
        Block::Table { headers, rows, .. } => {
            let hdr: usize = headers.iter().map(|h| h.len() + 3).sum();
            let body: usize = rows
                .iter()
                .flat_map(|r| r.iter().map(|c| c.len() + 3))
                .sum();
            hdr + body + headers.len() * 4 // separator line
        }
        Block::ThematicBreak => 4,
        Block::Image { alt, url, .. } => alt.len() + url.len() + 5,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_split_when_under_limit() {
        let ir = MessageIR {
            blocks: vec![Block::Paragraph(RichText::plain("hello"))],
        };
        let chunks = chunk_ir(&ir, 1000);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn splits_across_blocks() {
        let ir = MessageIR {
            blocks: vec![
                Block::Paragraph(RichText::plain("a".repeat(100))),
                Block::Paragraph(RichText::plain("b".repeat(100))),
                Block::Paragraph(RichText::plain("c".repeat(100))),
            ],
        };
        let chunks = chunk_ir(&ir, 120);
        assert!(
            chunks.len() >= 2,
            "expected split, got {} chunks",
            chunks.len()
        );
    }

    #[test]
    fn code_block_never_split() {
        let ir = MessageIR {
            blocks: vec![
                Block::Paragraph(RichText::plain("intro")),
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "x".repeat(200),
                },
            ],
        };
        let chunks = chunk_ir(&ir, 50);
        // Code block should be intact in one of the chunks
        let has_code = chunks.iter().any(|c| {
            c.blocks
                .iter()
                .any(|b| matches!(b, Block::CodeBlock { .. }))
        });
        assert!(has_code);
    }

    #[test]
    fn empty_ir_returns_clone() {
        let ir = MessageIR::new();
        let chunks = chunk_ir(&ir, 100);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].blocks.is_empty());
    }
}
