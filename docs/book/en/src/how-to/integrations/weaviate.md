# Weaviate

[Weaviate](https://weaviate.io/) is a cloud-native, open-source vector database with built-in support for hybrid search and multi-tenancy. `synaptic-weaviate` implements the [`VectorStore`] trait using the Weaviate v1 REST API.

## Setup

```toml
[dependencies]
synaptic = { version = "0.2", features = ["weaviate"] }
```

Run a local Weaviate instance with Docker:

```bash
docker run -d -p 8080:8080 -p 50051:50051 cr.weaviate.io/semitechnologies/weaviate:latest
```

Or use [Weaviate Cloud Services](https://console.weaviate.cloud/).

## Configuration

```rust,ignore
use synaptic::weaviate::{WeaviateVectorStore, WeaviateConfig};

// Local Weaviate
let config = WeaviateConfig::new("http", "localhost:8080", "Documents");

// Weaviate Cloud Services (WCS) with API key
let config = WeaviateConfig::new("https", "my-cluster.weaviate.network", "Documents")
    .with_api_key("wcs-secret-key");

let store = WeaviateVectorStore::new(config);

// Create class schema (idempotent — safe to call multiple times)
store.initialize().await?;
```

### Configuration reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `scheme` | `String` | required | `"http"` or `"https"` |
| `host` | `String` | required | Host and port (e.g. `localhost:8080`) |
| `class_name` | `String` | required | Weaviate class name (must start with uppercase) |
| `api_key` | `Option<String>` | `None` | API key for WCS authentication |

## Adding documents

```rust,ignore
use synaptic::weaviate::{WeaviateVectorStore, WeaviateConfig};
use synaptic::core::Document;
use synaptic::openai::OpenAiEmbeddings;
use std::sync::Arc;

let config = WeaviateConfig::new("http", "localhost:8080", "Articles");
let store = WeaviateVectorStore::new(config);
store.initialize().await?;

let embeddings = Arc::new(OpenAiEmbeddings::new(/* config */));

let docs = vec![
    Document::new("1", "Rust is a systems programming language."),
    Document::new("2", "Weaviate is a vector database."),
    Document::new("3", "Synaptic is a Rust agent framework."),
];

let ids = store.add_documents(docs, embeddings.as_ref()).await?;
println!("Added {} documents", ids.len());
```

## Similarity search

```rust,ignore
use synaptic::core::VectorStore;

let results = store.similarity_search("systems programming", 3, embeddings.as_ref()).await?;
for doc in results {
    println!("[{}] {}", doc.id, doc.content);
}
```

## Deleting documents

```rust,ignore
store.delete(&["weaviate-uuid-1".to_string(), "weaviate-uuid-2".to_string()]).await?;
```

## RAG pipeline

```rust,ignore
use synaptic::retrieval::VectorStoreRetriever;
use synaptic::core::Retriever;
use std::sync::Arc;

let store = Arc::new(WeaviateVectorStore::new(config));
let retriever = VectorStoreRetriever::new(store, embeddings, 4);

let docs = retriever.get_relevant_documents("Rust async programming").await?;
```

## Error handling

```rust,ignore
use synaptic::core::SynapticError;

match store.similarity_search("query", 5, embeddings.as_ref()).await {
    Ok(docs) => println!("Found {} results", docs.len()),
    Err(SynapticError::VectorStore(msg)) => eprintln!("Weaviate error: {msg}"),
    Err(e) => return Err(e.into()),
}
```

## Class schema

`initialize()` creates the following Weaviate class if it does not exist:

```json
{
  "class": "Documents",
  "vectorizer": "none",
  "properties": [
    { "name": "content",  "dataType": ["text"] },
    { "name": "docId",    "dataType": ["text"] },
    { "name": "metadata", "dataType": ["text"] }
  ]
}
```

Vectors are provided by Synaptic (no Weaviate vectorizer module needed).
