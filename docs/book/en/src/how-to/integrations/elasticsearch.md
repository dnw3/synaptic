# Elasticsearch Vector Store

This guide shows how to use [Elasticsearch](https://www.elastic.co/elasticsearch) as a vector store backend in Synaptic. Elasticsearch supports approximate kNN (k-nearest neighbors) search using dense vector fields.

## Setup

Add the `elasticsearch` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "elasticsearch"] }
```

Start an Elasticsearch instance (e.g. via Docker):

```bash
docker run -p 9200:9200 -e "discovery.type=single-node" -e "xpack.security.enabled=false" \
  docker.elastic.co/elasticsearch/elasticsearch:8.12.0
```

## Configuration

Create an `ElasticsearchConfig` with the server URL, index name, and vector dimensionality:

```rust,ignore
use synaptic::elasticsearch::{ElasticsearchConfig, ElasticsearchVectorStore};

let config = ElasticsearchConfig::new("http://localhost:9200", "my_index", 1536);
let store = ElasticsearchVectorStore::new(config);
```

### Authentication

For secured Elasticsearch clusters, provide credentials:

```rust,ignore
let config = ElasticsearchConfig::new("https://es.example.com:9200", "my_index", 1536)
    .with_credentials("elastic", "changeme");
```

### Creating the index

Call `ensure_index()` to create the index with the appropriate kNN vector mapping if it does not already exist:

```rust,ignore
store.ensure_index().await?;
```

This creates an index with a `dense_vector` field configured for the specified dimensionality and cosine similarity. The call is idempotent.

### Similarity metric

The default similarity is cosine. You can change it:

```rust,ignore
let config = ElasticsearchConfig::new("http://localhost:9200", "my_index", 1536)
    .with_similarity("dot_product");
```

Available options: `"cosine"` (default), `"dot_product"`, `"l2_norm"`.

## Adding documents

`ElasticsearchVectorStore` implements the `VectorStore` trait:

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

## Index Mapping Configuration

While `ensure_index()` creates a default mapping automatically, you may want full control over the index mapping for production use. Below is the recommended Elasticsearch mapping for vector search:

```json
{
  "mappings": {
    "properties": {
      "embedding": {
        "type": "dense_vector",
        "dims": 1536,
        "index": true,
        "similarity": "cosine"
      },
      "content": { "type": "text" },
      "metadata": { "type": "object", "enabled": true }
    }
  }
}
```

### Creating the index via the REST API

You can create the index with a custom mapping using the Elasticsearch REST API:

```bash
curl -X PUT "http://localhost:9200/my-index" \
  -H "Content-Type: application/json" \
  -d '{
    "mappings": {
      "properties": {
        "embedding": {
          "type": "dense_vector",
          "dims": 1536,
          "index": true,
          "similarity": "cosine"
        },
        "content": { "type": "text" },
        "metadata": { "type": "object", "enabled": true }
      }
    }
  }'
```

### Key mapping fields

- **`type: "dense_vector"`** -- Tells Elasticsearch this field stores a fixed-length float array for vector operations.
- **`dims`** -- Must match the dimensionality of your embedding model (e.g. 1536 for `text-embedding-3-small`, 768 for many open-source models).
- **`index: true`** -- Enables the kNN search data structure. Without this, you can store vectors but cannot perform efficient approximate nearest-neighbor queries. Set to `true` for production use.
- **`similarity`** -- Determines the distance function used for kNN search:
  - `"cosine"` (default) -- Cosine similarity, recommended for most embedding models.
  - `"dot_product"` -- Dot product, best for unit-length normalized vectors.
  - `"l2_norm"` -- Euclidean distance.

### Mapping for metadata filtering

If you plan to filter search results by metadata fields, add explicit mappings for those fields:

```json
{
  "mappings": {
    "properties": {
      "embedding": {
        "type": "dense_vector",
        "dims": 1536,
        "index": true,
        "similarity": "cosine"
      },
      "content": { "type": "text" },
      "metadata": {
        "properties": {
          "source": { "type": "keyword" },
          "category": { "type": "keyword" },
          "created_at": { "type": "date" }
        }
      }
    }
  }
}
```

Using `keyword` type for metadata fields enables exact-match filtering in kNN queries.

## RAG Pipeline Example

Below is a complete Retrieval-Augmented Generation (RAG) pipeline that loads documents, splits them, embeds and stores them in Elasticsearch, then retrieves relevant context to answer a question.

```rust,ignore
use std::sync::Arc;
use synaptic::core::{
    ChatModel, ChatRequest, Document, Embeddings, Message, Retriever, VectorStore,
};
use synaptic::elasticsearch::{ElasticsearchConfig, ElasticsearchVectorStore};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::splitters::RecursiveCharacterTextSplitter;
use synaptic::vectorstores::VectorStoreRetriever;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure embeddings and LLM
    let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
    let llm = OpenAiChatModel::new("gpt-4o-mini");

    // 2. Connect to Elasticsearch and create the index
    let config = ElasticsearchConfig::new("http://localhost:9200", "rag_documents", 1536);
    let store = ElasticsearchVectorStore::new(config);
    store.ensure_index().await?;

    // 3. Load and split documents
    let raw_docs = vec![
        Document::new("doc1", "Rust is a multi-paradigm, general-purpose programming language \
            that emphasizes performance, type safety, and concurrency. It enforces memory safety \
            without a garbage collector."),
        Document::new("doc2", "Elasticsearch is a distributed, RESTful search and analytics engine. \
            It supports vector search through dense_vector fields and approximate kNN queries, \
            making it suitable for semantic search and RAG applications."),
    ];

    let splitter = RecursiveCharacterTextSplitter::new(500, 50);
    let chunks = splitter.split_documents(&raw_docs);

    // 4. Embed and store in Elasticsearch
    store.add_documents(chunks, embeddings.as_ref()).await?;

    // 5. Create a retriever
    let store = Arc::new(store);
    let retriever = VectorStoreRetriever::new(store, embeddings, 3);

    // 6. Retrieve relevant context
    let query = "What is Rust?";
    let relevant_docs = retriever.retrieve(query, 3).await?;

    let context = relevant_docs
        .iter()
        .map(|doc| doc.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    // 7. Generate answer using retrieved context
    let messages = vec![
        Message::system("Answer the user's question based on the following context. \
            If the context doesn't contain relevant information, say so.\n\n\
            Context:\n{context}".replace("{context}", &context)),
        Message::human(query),
    ];

    let response = llm.chat(ChatRequest::new(messages)).await?;
    println!("Answer: {}", response.message.content());

    Ok(())
}
```

## Configuration reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | required | Elasticsearch server URL |
| `index_name` | `String` | required | Name of the Elasticsearch index |
| `dims` | `u32` | required | Dimensionality of embedding vectors |
| `username` | `Option<String>` | `None` | Username for basic auth |
| `password` | `Option<String>` | `None` | Password for basic auth |
| `similarity` | `String` | `"cosine"` | Similarity metric (`cosine`, `dot_product`, `l2_norm`) |
