# Loaders & Vector Store

## LarkDocLoader

Load Feishu documents and Wiki pages into Synaptic [`Document`]s for RAG pipelines.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDocLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");

// Load specific document tokens
let loader = LarkDocLoader::new(config.clone())
    .with_doc_tokens(vec!["doxcnAbcXxx".to_string()]);

// Or traverse an entire Wiki space
let loader = LarkDocLoader::new(config)
    .with_wiki_space_id("spcXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("Title: {}", doc.metadata["title"]);
    println!("URL:   {}", doc.metadata["url"]);
    println!("Length: {} chars", doc.content.len());
}
```

### Document Metadata

Each document includes:

| Field | Description |
|-------|-------------|
| `doc_id` | The Feishu document token |
| `title` | Document title |
| `source` | `lark:doc:<token>` |
| `url` | Direct Feishu document URL |
| `doc_type` | Always `"docx"` |

### Builder Options

| Method | Description |
|--------|-------------|
| `with_doc_tokens(tokens)` | Load specific document tokens |
| `with_wiki_space_id(id)` | Traverse all docs in a Wiki space |

---

## LarkWikiLoader

Recursively load all pages from a Feishu Wiki space as `Document`s. The `with_space_id` and `with_max_depth` builder methods control which space is traversed and how deep to recurse.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkWikiLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkWikiLoader::new(config)
    .with_space_id("spcXxx")
    .with_max_depth(3);

let docs = loader.load().await?;
println!("Loaded {} Wiki pages", docs.len());
```

---

## LarkDriveLoader

Load files from a Feishu Drive folder, automatically dispatching to the appropriate sub-loader (doc, spreadsheet, etc.) based on file type.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDriveLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkDriveLoader::new(config, "fldcnXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("{}: {} chars", doc.metadata["file_name"], doc.content.len());
}
```

---

## LarkSpreadsheetLoader

Load rows from a Feishu spreadsheet as Synaptic [`Document`]s. Each row becomes one document; column headers are stored as metadata keys.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkSpreadsheetLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkSpreadsheetLoader::new(config, "shtcnXxx", "0");

let docs = loader.load().await?;
for doc in &docs {
    println!("Row: {}", doc.content);
    println!("Sheet: {}", doc.metadata["sheet_id"]);
}
```

---

## LarkVectorStore

Store and search vectors using the Feishu Search API as the backend. Feishu handles embedding on the server side; your documents are indexed in Lark and retrieved via semantic search.

```rust,ignore
use synaptic::lark::{LarkConfig, LarkVectorStore};
use synaptic::core::VectorStore;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let store = LarkVectorStore::new(config, "data_source_id_xxx");

// Index documents
store.add_documents(docs).await?;

// Semantic search — embedding is handled by the Feishu platform
let results = store.similarity_search("quarterly earnings", 5).await?;
for doc in &results {
    println!("{}", doc.content);
}
```
