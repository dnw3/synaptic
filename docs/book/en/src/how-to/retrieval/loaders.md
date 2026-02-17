# Document Loaders

This guide shows how to load documents from various sources using Synapse's `Loader` trait and its built-in implementations.

## Overview

Every loader implements the `Loader` trait from `synapse_loaders`:

```rust
#[async_trait]
pub trait Loader: Send + Sync {
    async fn load(&self) -> Result<Vec<Document>, SynapseError>;
}
```

Each loader returns `Vec<Document>`. A `Document` has three fields:

- `id: String` -- a unique identifier
- `content: String` -- the document text
- `metadata: HashMap<String, Value>` -- arbitrary key-value metadata

## TextLoader

Wraps a string of text into a single `Document`. Useful when you already have content in memory.

```rust
use synapse_loaders::{TextLoader, Loader};

let loader = TextLoader::new("doc-1", "Rust is a systems programming language.");
let docs = loader.load().await?;

assert_eq!(docs.len(), 1);
assert_eq!(docs[0].content, "Rust is a systems programming language.");
```

The first argument is the document ID; the second is the content.

## FileLoader

Reads a file from disk using `tokio::fs::read_to_string` and returns a single `Document`. The file path is used as the document ID, and a `source` metadata key is set to the file path.

```rust
use synapse_loaders::{FileLoader, Loader};

let loader = FileLoader::new("data/notes.txt");
let docs = loader.load().await?;

assert_eq!(docs[0].metadata["source"], "data/notes.txt");
```

## JsonLoader

Loads documents from a JSON string. If the JSON is an array of objects, each object becomes a `Document`. If it is a single object, one `Document` is produced.

```rust
use synapse_loaders::{JsonLoader, Loader};

let json_data = r#"[
    {"id": "1", "content": "First document"},
    {"id": "2", "content": "Second document"}
]"#;

let loader = JsonLoader::new(json_data);
let docs = loader.load().await?;

assert_eq!(docs.len(), 2);
assert_eq!(docs[0].content, "First document");
```

By default, `JsonLoader` looks for `"id"` and `"content"` keys. You can customize them with builder methods:

```rust
let loader = JsonLoader::new(json_data)
    .with_id_key("doc_id")
    .with_content_key("text");
```

## CsvLoader

Loads documents from CSV data. Each row becomes a `Document`. All columns are stored as metadata.

```rust
use synapse_loaders::{CsvLoader, Loader};

let csv_data = "title,body,author\nIntro,Hello world,Alice\nChapter 1,Once upon a time,Bob";

let loader = CsvLoader::new(csv_data)
    .with_content_column("body")
    .with_id_column("title");

let docs = loader.load().await?;

assert_eq!(docs.len(), 2);
assert_eq!(docs[0].id, "Intro");
assert_eq!(docs[0].content, "Hello world");
assert_eq!(docs[0].metadata["author"], "Alice");
```

If no `content_column` is specified, all columns are concatenated. If no `id_column` is specified, IDs default to `"row-0"`, `"row-1"`, etc.

## DirectoryLoader

Loads all files from a directory, each file becoming a `Document`. Use `with_glob` to filter by extension and `with_recursive` to include subdirectories.

```rust
use synapse_loaders::{DirectoryLoader, Loader};

let loader = DirectoryLoader::new("./docs")
    .with_glob("*.txt")
    .with_recursive(true);

let docs = loader.load().await?;
// Each document has a `source` metadata key set to the file path
```

Document IDs are the relative file paths from the base directory.

## MarkdownLoader

Reads a markdown file and returns it as a single `Document` with `format: "markdown"` in metadata.

```rust
use synapse_loaders::{MarkdownLoader, Loader};

let loader = MarkdownLoader::new("docs/guide.md");
let docs = loader.load().await?;

assert_eq!(docs[0].metadata["format"], "markdown");
```

## WebBaseLoader

Fetches content from a URL via HTTP GET and returns a single `Document`. Metadata includes `source` (the URL) and `content_type` (from the response header).

```rust
use synapse_loaders::{WebBaseLoader, Loader};

let loader = WebBaseLoader::new("https://example.com/page.html");
let docs = loader.load().await?;

assert_eq!(docs[0].metadata["source"], "https://example.com/page.html");
```

## Lazy loading

Every `Loader` also provides a `lazy_load()` method that returns a `Stream` of documents instead of loading all at once. The default implementation wraps `load()`, but custom loaders can override it for true lazy behavior.

```rust
use futures::StreamExt;
use synapse_loaders::{DirectoryLoader, Loader};

let loader = DirectoryLoader::new("./data").with_glob("*.txt");
let mut stream = loader.lazy_load();

while let Some(result) = stream.next().await {
    let doc = result?;
    println!("Loaded: {}", doc.id);
}
```
