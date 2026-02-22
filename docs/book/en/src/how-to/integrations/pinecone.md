# Pinecone Vector Store

This guide shows how to use [Pinecone](https://www.pinecone.io/) as a vector store backend in Synaptic. Pinecone is a managed vector database built for real-time similarity search at scale.

## Setup

Add the `pinecone` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "pinecone"] }
```

Set your Pinecone API key:

```bash
export PINECONE_API_KEY="your-pinecone-api-key"
```

You also need an existing Pinecone index. Create one through the [Pinecone console](https://app.pinecone.io/) or the Pinecone API. Note the **index host URL** (e.g. `https://my-index-abc123.svc.aped-1234.pinecone.io`).

## Configuration

Create a `PineconeConfig` with your API key and index host URL:

```rust,ignore
use synaptic::pinecone::{PineconeConfig, PineconeVectorStore};

let config = PineconeConfig::new("your-pinecone-api-key", "https://my-index-abc123.svc.aped-1234.pinecone.io");
let store = PineconeVectorStore::new(config);
```

### Namespace

Pinecone supports namespaces for partitioning data within an index:

```rust,ignore
let config = PineconeConfig::new("api-key", "https://my-index.pinecone.io")
    .with_namespace("production");
```

If no namespace is set, the default namespace is used.

## Adding documents

`PineconeVectorStore` implements the `VectorStore` trait. Pass an embeddings provider to compute vectors:

```rust,ignore
use synaptic::core::{VectorStore, Document, Embeddings};
use synaptic::openai::OpenAiEmbeddings;

let embeddings = OpenAiEmbeddings::new("text-embedding-3-small");

let docs = vec![
    Document::new("1", "Rust is a systems programming language"),
    Document::new("2", "Python is great for data science"),
    Document::new("3", "Go is designed for concurrency"),
];

let ids = store.add_documents(docs, &embeddings).await?;
```

## Similarity search

Find the `k` most similar documents to a text query:

```rust,ignore
let results = store.similarity_search("fast systems language", 3, &embeddings).await?;
for doc in &results {
    println!("{}: {}", doc.id, doc.content);
}
```

### Search with scores

```rust,ignore
let scored = store.similarity_search_with_score("concurrency", 3, &embeddings).await?;
for (doc, score) in &scored {
    println!("{} (score: {:.3}): {}", doc.id, score, doc.content);
}
```

## Deleting documents

Remove documents by their IDs:

```rust,ignore
store.delete(&["1", "3"]).await?;
```

## Using with a retriever

Wrap the store in a `VectorStoreRetriever` for use with Synaptic's retrieval infrastructure:

```rust,ignore
use std::sync::Arc;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::core::Retriever;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = Arc::new(store);

let retriever = VectorStoreRetriever::new(store, embeddings, 5);
let results = retriever.retrieve("fast language", 5).await?;
```

## Namespace Isolation

Namespaces are a common pattern for building multi-tenant RAG applications with Pinecone. Each tenant's data lives in a separate namespace within the same index, providing logical isolation without the overhead of managing multiple indexes.

```rust,ignore
use synaptic::pinecone::{PineconeConfig, PineconeVectorStore};
use synaptic::core::{VectorStore, Document, Embeddings};
use synaptic::openai::OpenAiEmbeddings;

let api_key = std::env::var("PINECONE_API_KEY")?;
let index_host = "https://my-index-abc123.svc.aped-1234.pinecone.io";

// Create stores with different namespaces for tenant isolation
let config_a = PineconeConfig::new(&api_key, index_host)
    .with_namespace("tenant-a");
let config_b = PineconeConfig::new(&api_key, index_host)
    .with_namespace("tenant-b");

let store_a = PineconeVectorStore::new(config_a);
let store_b = PineconeVectorStore::new(config_b);

let embeddings = OpenAiEmbeddings::new("text-embedding-3-small");

// Tenant A's documents are invisible to Tenant B
let docs_a = vec![Document::new("a1", "Tenant A internal report")];
store_a.add_documents(docs_a, &embeddings).await?;

// Searching in Tenant B's namespace returns no results from Tenant A
let results = store_b.similarity_search("internal report", 5, &embeddings).await?;
assert!(results.is_empty());
```

This approach scales well because Pinecone handles namespace-level partitioning internally. You can add, search, and delete documents in one namespace without affecting others.

## RAG Pipeline Example

A complete RAG pipeline: load documents, split them into chunks, embed and store in Pinecone, then retrieve relevant context and generate an answer.

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message, Embeddings, VectorStore, Retriever};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::pinecone::{PineconeConfig, PineconeVectorStore};
use synaptic::splitters::RecursiveCharacterTextSplitter;
use synaptic::loaders::TextLoader;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let backend = Arc::new(HttpBackend::new());
let embeddings = Arc::new(OpenAiEmbeddings::new(
    OpenAiEmbeddings::config("text-embedding-3-small"),
    backend.clone(),
));

// 1. Load and split
let loader = TextLoader::new("docs/knowledge-base.txt");
let docs = loader.load().await?;
let splitter = RecursiveCharacterTextSplitter::new(500, 50);
let chunks = splitter.split_documents(&docs)?;

// 2. Store in Pinecone
let config = PineconeConfig::new(
    std::env::var("PINECONE_API_KEY")?,
    "https://my-index-abc123.svc.aped-1234.pinecone.io",
);
let store = PineconeVectorStore::new(config);
store.add_documents(chunks, embeddings.as_ref()).await?;

// 3. Retrieve and answer
let store = Arc::new(store);
let retriever = VectorStoreRetriever::new(store, embeddings.clone(), 5);
let relevant = retriever.retrieve("What is Synaptic?", 5).await?;
let context = relevant.iter().map(|d| d.content.as_str()).collect::<Vec<_>>().join("\n\n");

let model = OpenAiChatModel::new(/* config */);
let request = ChatRequest::new(vec![
    Message::system(&format!("Answer based on context:\n{context}")),
    Message::human("What is Synaptic?"),
]);
let response = model.chat(&request).await?;
```

## Configuration reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | `String` | required | Pinecone API key |
| `host` | `String` | required | Index host URL from the Pinecone console |
| `namespace` | `Option<String>` | `None` | Namespace for data partitioning |
