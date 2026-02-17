# Self-Query Retriever

This guide shows how to use the `SelfQueryRetriever` to automatically extract structured metadata filters from natural language queries.

## Overview

Users often express search intent that includes both a semantic query and metadata constraints in the same sentence. For example:

> "Find documents about Rust published after 2024"

This contains:
- A **semantic query**: "documents about Rust"
- A **metadata filter**: `year > 2024`

The `SelfQueryRetriever` uses a `ChatModel` to parse the user's natural language query into a structured search query plus metadata filters, then applies those filters to the results from a base retriever.

## Defining metadata fields

First, describe the metadata fields available in your document corpus using `MetadataFieldInfo`:

```rust
use synapse_retrieval::MetadataFieldInfo;

let fields = vec![
    MetadataFieldInfo {
        name: "year".to_string(),
        description: "The year the document was published".to_string(),
        field_type: "integer".to_string(),
    },
    MetadataFieldInfo {
        name: "language".to_string(),
        description: "The programming language discussed".to_string(),
        field_type: "string".to_string(),
    },
    MetadataFieldInfo {
        name: "author".to_string(),
        description: "The author of the document".to_string(),
        field_type: "string".to_string(),
    },
];
```

Each field has a `name`, a human-readable `description`, and a `field_type` that tells the LLM what kind of values to expect.

## Basic usage

```rust
use std::sync::Arc;
use synapse_retrieval::{SelfQueryRetriever, MetadataFieldInfo, Retriever};

let base_retriever: Arc<dyn Retriever> = Arc::new(/* any retriever */);
let model: Arc<dyn ChatModel> = Arc::new(/* any ChatModel */);

let retriever = SelfQueryRetriever::new(base_retriever, model, fields);

let results = retriever.retrieve(
    "find articles about Rust written by Alice",
    5,
).await?;
// LLM extracts: query="Rust", filters: [language eq "Rust", author eq "Alice"]
```

## How it works

1. The retriever builds a prompt describing the available metadata fields and sends the user's query to the LLM.
2. The LLM responds with a JSON object containing:
   - `"query"` -- the extracted semantic search query.
   - `"filters"` -- an array of filter objects, each with `"field"`, `"op"`, and `"value"`.
3. The retriever runs the extracted query through the base retriever (fetching extra candidates, `top_k * 2`).
4. Filters are applied to the results, keeping only documents whose metadata matches all filter conditions.
5. The final filtered results are truncated to `top_k` and returned.

## Supported filter operators

| Operator | Meaning |
|----------|---------|
| `eq` | Equal to |
| `gt` | Greater than |
| `gte` | Greater than or equal to |
| `lt` | Less than |
| `lte` | Less than or equal to |
| `contains` | String contains substring |

Numeric comparisons work on both integers and floats. String comparisons use lexicographic ordering.

## Full example

```rust
use std::sync::Arc;
use std::collections::HashMap;
use synapse_retrieval::{
    BM25Retriever,
    SelfQueryRetriever,
    MetadataFieldInfo,
    Document,
    Retriever,
};
use serde_json::json;

// Documents with metadata
let docs = vec![
    Document::with_metadata(
        "1",
        "An introduction to Rust's ownership model",
        HashMap::from([
            ("year".to_string(), json!(2024)),
            ("language".to_string(), json!("Rust")),
        ]),
    ),
    Document::with_metadata(
        "2",
        "Advanced Python patterns for data pipelines",
        HashMap::from([
            ("year".to_string(), json!(2023)),
            ("language".to_string(), json!("Python")),
        ]),
    ),
    Document::with_metadata(
        "3",
        "Rust async programming with Tokio",
        HashMap::from([
            ("year".to_string(), json!(2025)),
            ("language".to_string(), json!("Rust")),
        ]),
    ),
];

let base = Arc::new(BM25Retriever::new(docs));
let model: Arc<dyn ChatModel> = Arc::new(/* your model */);

let fields = vec![
    MetadataFieldInfo {
        name: "year".to_string(),
        description: "Publication year".to_string(),
        field_type: "integer".to_string(),
    },
    MetadataFieldInfo {
        name: "language".to_string(),
        description: "Programming language topic".to_string(),
        field_type: "string".to_string(),
    },
];

let retriever = SelfQueryRetriever::new(base, model, fields);

// Natural language query with implicit filters
let results = retriever.retrieve("Rust articles from 2025", 5).await?;
// LLM extracts: query="Rust articles", filters: [language eq "Rust", year eq 2025]
// Returns only document 3
```

## Considerations

- The quality of filter extraction depends on the LLM. Use a capable model for reliable results.
- Only filters referencing fields declared in `MetadataFieldInfo` are applied; unknown fields are ignored.
- If the LLM cannot parse the query into structured filters, it falls back to an empty filter list and returns standard retrieval results.
