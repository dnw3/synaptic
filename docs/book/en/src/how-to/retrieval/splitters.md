# Text Splitters

This guide shows how to break large documents into smaller chunks using Synapse's `TextSplitter` trait and its built-in implementations.

## Overview

All splitters implement the `TextSplitter` trait from `synapse_splitters`:

```rust
pub trait TextSplitter: Send + Sync {
    fn split_text(&self, text: &str) -> Vec<String>;
    fn split_documents(&self, docs: Vec<Document>) -> Vec<Document>;
}
```

- `split_text()` takes a string and returns a vector of chunks.
- `split_documents()` splits each document's content, producing new `Document` values with preserved metadata and an added `chunk_index` field.

## CharacterTextSplitter

Splits text on a single separator string, then merges small pieces to stay under `chunk_size`.

```rust
use synapse_splitters::CharacterTextSplitter;
use synapse_splitters::TextSplitter;

// Chunk size in characters, default separator is "\n\n"
let splitter = CharacterTextSplitter::new(500);
let chunks = splitter.split_text("long text...");
```

Configure the separator and overlap:

```rust
let splitter = CharacterTextSplitter::new(500)
    .with_separator("\n")       // Split on single newlines
    .with_chunk_overlap(50);    // 50 characters of overlap between chunks
```

## RecursiveCharacterTextSplitter

The most commonly used splitter. Tries a hierarchy of separators in order, splitting with the first one that produces chunks small enough. If a chunk is still too large, it recurses with the next separator.

Default separators: `["\n\n", "\n", " ", ""]`

```rust
use synapse_splitters::RecursiveCharacterTextSplitter;
use synapse_splitters::TextSplitter;

let splitter = RecursiveCharacterTextSplitter::new(1000)
    .with_chunk_overlap(200);

let chunks = splitter.split_text("long document text...");
```

Custom separators:

```rust
let splitter = RecursiveCharacterTextSplitter::new(1000)
    .with_separators(vec![
        "\n\n\n".to_string(),
        "\n\n".to_string(),
        "\n".to_string(),
        " ".to_string(),
        String::new(),
    ]);
```

### Language-aware splitting

Use `from_language()` to get separators tuned for a specific programming language:

```rust
use synapse_splitters::{RecursiveCharacterTextSplitter, Language};

let splitter = RecursiveCharacterTextSplitter::from_language(
    Language::Rust,
    1000,  // chunk_size
    200,   // chunk_overlap
);
```

## MarkdownHeaderTextSplitter

Splits markdown text by headers, adding the header hierarchy to each chunk's metadata.

```rust
use synapse_splitters::{MarkdownHeaderTextSplitter, HeaderType};

let splitter = MarkdownHeaderTextSplitter::new(vec![
    HeaderType { level: "#".to_string(), name: "h1".to_string() },
    HeaderType { level: "##".to_string(), name: "h2".to_string() },
    HeaderType { level: "###".to_string(), name: "h3".to_string() },
]);

let docs = splitter.split_markdown("# Title\n\nIntro text\n\n## Section\n\nBody text");
// docs[0].metadata contains {"h1": "Title"}
// docs[1].metadata contains {"h1": "Title", "h2": "Section"}
```

A convenience constructor provides the default `#`, `##`, `###` configuration:

```rust
let splitter = MarkdownHeaderTextSplitter::default_headers();
```

Note that `MarkdownHeaderTextSplitter` also implements `TextSplitter`, but `split_markdown()` returns `Vec<Document>` with full metadata, which is usually what you want.

## TokenTextSplitter

Splits text by estimated token count using a ~4 characters per token heuristic. Splits at word boundaries to keep chunks readable.

```rust
use synapse_splitters::TokenTextSplitter;
use synapse_splitters::TextSplitter;

// chunk_size is in estimated tokens (not characters)
let splitter = TokenTextSplitter::new(500)
    .with_chunk_overlap(50);

let chunks = splitter.split_text("long text...");
```

This is consistent with the token estimation used in `ConversationTokenBufferMemory`.

## Splitting documents

All splitters can split a `Vec<Document>` into smaller chunks. Each chunk inherits the parent's metadata and gets a `chunk_index` field. The chunk ID is formatted as `"{original_id}-chunk-{index}"`.

```rust
use synapse_splitters::{RecursiveCharacterTextSplitter, TextSplitter};
use synapse_retrieval::Document;

let splitter = RecursiveCharacterTextSplitter::new(500);

let docs = vec![
    Document::new("doc-1", "A very long document..."),
    Document::new("doc-2", "Another long document..."),
];

let chunks = splitter.split_documents(docs);
// chunks[0].id == "doc-1-chunk-0"
// chunks[0].metadata["chunk_index"] == 0
```

## Choosing a splitter

| Splitter | Best for |
|----------|----------|
| `CharacterTextSplitter` | Simple splitting on a known delimiter |
| `RecursiveCharacterTextSplitter` | General-purpose text -- tries to preserve paragraphs, then sentences, then words |
| `MarkdownHeaderTextSplitter` | Markdown documents where you want header context in metadata |
| `TokenTextSplitter` | When you need to control chunk size in tokens rather than characters |
