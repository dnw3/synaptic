# Chroma Vector Store

This guide shows how to use [Chroma](https://www.trychroma.com/) as a vector store backend in Synaptic. Chroma is an open-source embedding database that runs locally or in the cloud.

## Setup

Add the `chroma` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "chroma"] }
```

Start a Chroma server (e.g. via Docker):

```bash
docker run -p 8000:8000 chromadb/chroma
```

## Configuration

Create a `ChromaConfig` with the server URL and collection name:

```rust,ignore
use synaptic::chroma::{ChromaConfig, ChromaVectorStore};

let config = ChromaConfig::new("http://localhost:8000", "my_collection");
let store = ChromaVectorStore::new(config);
```

The default URL is `http://localhost:8000`.

### Creating the collection

Call `ensure_collection()` to create the collection if it does not already exist. This is idempotent and safe to call on every startup:

```rust,ignore
store.ensure_collection().await?;
```

### Authentication

If your Chroma server requires authentication, pass credentials:

```rust,ignore
let config = ChromaConfig::new("https://chroma.example.com", "my_collection")
    .with_auth_token("your-token");
```

## Adding documents

`ChromaVectorStore` implements the `VectorStore` trait:

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

Find the `k` most similar documents:

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

Wrap the store in a `VectorStoreRetriever`:

```rust,ignore
use std::sync::Arc;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::core::Retriever;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = Arc::new(store);

let retriever = VectorStoreRetriever::new(store, embeddings, 5);
let results = retriever.retrieve("fast language", 5).await?;
```

## Docker Deployment

Chroma is easy to deploy with Docker for both development and production environments.

**Quick start** -- run a Chroma server with default settings:

```bash
# Start Chroma on port 8000
docker run -p 8000:8000 chromadb/chroma:latest
```

**With persistent storage** -- mount a volume so data survives container restarts:

```bash
docker run -p 8000:8000 -v ./chroma-data:/chroma/chroma chromadb/chroma:latest
```

**Docker Compose** -- for production deployments, use a `docker-compose.yml`:

```yaml
version: "3.8"
services:
  chroma:
    image: chromadb/chroma:latest
    ports:
      - "8000:8000"
    volumes:
      - chroma-data:/chroma/chroma
    restart: unless-stopped

volumes:
  chroma-data:
```

Then connect from Synaptic:

```rust,ignore
use synaptic::chroma::{ChromaConfig, ChromaVectorStore};

let config = ChromaConfig::new("http://localhost:8000", "my_collection");
let store = ChromaVectorStore::new(config);
store.ensure_collection().await?;
```

For remote or authenticated deployments, use `with_auth_token()`:

```rust,ignore
let config = ChromaConfig::new("https://chroma.example.com", "my_collection")
    .with_auth_token("your-token");
```

## RAG Pipeline Example

A complete RAG pipeline: load documents, split them into chunks, embed and store in Chroma, then retrieve relevant context and generate an answer.

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message, Embeddings, VectorStore, Retriever};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::chroma::{ChromaConfig, ChromaVectorStore};
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

// 2. Store in Chroma
let config = ChromaConfig::new("http://localhost:8000", "my_collection");
let store = ChromaVectorStore::new(config);
store.ensure_collection().await?;
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
| `url` | `String` | `"http://localhost:8000"` | Chroma server URL |
| `collection_name` | `String` | required | Name of the collection |
| `auth_token` | `Option<String>` | `None` | Authentication token |
