# MongoDB Atlas Vector Search

This guide shows how to use [MongoDB Atlas Vector Search](https://www.mongodb.com/products/platform/atlas-vector-search) as a vector store backend in Synaptic. Atlas Vector Search enables semantic similarity search on data stored in MongoDB.

## Setup

Add the `mongodb` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "mongodb"] }
```

### Prerequisites

1. A MongoDB Atlas cluster (M10 or higher, or a free shared cluster with Atlas Search enabled).
2. A **vector search index** configured on the target collection. Create one via the Atlas UI or the Atlas Admin API.

Example index definition (JSON):

```json
{
  "type": "vectorSearch",
  "fields": [
    {
      "type": "vector",
      "path": "embedding",
      "numDimensions": 1536,
      "similarity": "cosine"
    }
  ]
}
```

## Configuration

Create a `MongoVectorConfig` with the database name, collection name, index name, and vector dimensionality:

```rust,ignore
use synaptic::mongodb::{MongoVectorConfig, MongoVectorStore};

let config = MongoVectorConfig::new("my_database", "my_collection", "vector_index", 1536);
let store = MongoVectorStore::from_uri("mongodb+srv://user:pass@cluster.mongodb.net/", config).await?;
```

The `from_uri` constructor connects to MongoDB and is async.

### Embedding field name

By default, vectors are stored in a field called `"embedding"`. You can change this:

```rust,ignore
let config = MongoVectorConfig::new("mydb", "docs", "vector_index", 1536)
    .with_embedding_field("vector");
```

Make sure this matches the `path` in your Atlas vector search index definition.

### Content and metadata fields

Customize which fields store the document content and metadata:

```rust,ignore
let config = MongoVectorConfig::new("mydb", "docs", "vector_index", 1536)
    .with_content_field("text")
    .with_metadata_field("meta");
```

## Adding documents

`MongoVectorStore` implements the `VectorStore` trait:

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

## Atlas Search Index Setup

Before you can run similarity searches, you must create a **vector search index** on your MongoDB Atlas collection. This requires an **M10 or higher** dedicated cluster (vector search is not available on free/shared tier clusters).

### Creating an index via the Atlas UI

1. Navigate to your cluster in the [MongoDB Atlas console](https://cloud.mongodb.com/).
2. Go to **Search** > **Create Search Index**.
3. Choose **JSON Editor** and select the target database and collection.
4. Paste the following index definition:

```json
{
  "fields": [
    {
      "type": "vector",
      "path": "embedding",
      "numDimensions": 1536,
      "similarity": "cosine"
    }
  ]
}
```

5. Name your index (e.g. `vector_index`) and click **Create Search Index**.

> **Note:** The `path` field must match the `embedding_field` configured in your `MongoVectorConfig`. If you customized it with `.with_embedding_field("vector")`, set `"path": "vector"` in the index definition. Similarly, adjust `numDimensions` to match your embedding model's output dimensionality.

### Creating an index via the Atlas CLI

You can also create the index programmatically using the [MongoDB Atlas CLI](https://www.mongodb.com/docs/atlas/cli/):

First, save the index definition to a file called `index.json`:

```json
{
  "fields": [
    {
      "type": "vector",
      "path": "embedding",
      "numDimensions": 1536,
      "similarity": "cosine"
    }
  ]
}
```

Then run:

```bash
atlas clusters search indexes create \
  --clusterName my-cluster \
  --db my_database \
  --collection my_collection \
  --file index.json
```

The index build runs asynchronously. You can check its status with:

```bash
atlas clusters search indexes list \
  --clusterName my-cluster \
  --db my_database \
  --collection my_collection
```

Wait until the status shows **READY** before running similarity searches.

### Similarity options

The `similarity` field in the index definition controls how vectors are compared:

| Value | Description |
|-------|-------------|
| `cosine` | Cosine similarity (default, good for normalized embeddings) |
| `euclidean` | Euclidean (L2) distance |
| `dotProduct` | Dot product (use with unit-length vectors) |

## RAG Pipeline Example

Below is a complete Retrieval-Augmented Generation (RAG) pipeline that loads documents, splits them, embeds and stores them in MongoDB Atlas, then retrieves relevant context to answer a question.

```rust,ignore
use std::sync::Arc;
use synaptic::core::{
    ChatModel, ChatRequest, Document, Embeddings, Message, Retriever, VectorStore,
};
use synaptic::mongodb::{MongoVectorConfig, MongoVectorStore};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::splitters::RecursiveCharacterTextSplitter;
use synaptic::vectorstores::VectorStoreRetriever;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure embeddings and LLM
    let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
    let llm = OpenAiChatModel::new("gpt-4o-mini");

    // 2. Connect to MongoDB Atlas
    let config = MongoVectorConfig::new("my_database", "documents", "vector_index", 1536);
    let store = MongoVectorStore::from_uri(
        "mongodb+srv://user:pass@cluster.mongodb.net/",
        config,
    )
    .await?;

    // 3. Load and split documents
    let raw_docs = vec![
        Document::new("doc1", "Rust is a multi-paradigm, general-purpose programming language \
            that emphasizes performance, type safety, and concurrency. It enforces memory safety \
            without a garbage collector."),
        Document::new("doc2", "MongoDB Atlas is a fully managed cloud database service. It provides \
            built-in vector search capabilities for AI applications, supporting cosine, euclidean, \
            and dot product similarity metrics."),
    ];

    let splitter = RecursiveCharacterTextSplitter::new(500, 50);
    let chunks = splitter.split_documents(&raw_docs);

    // 4. Embed and store in MongoDB
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
| `database` | `String` | required | MongoDB database name |
| `collection` | `String` | required | MongoDB collection name |
| `index_name` | `String` | required | Atlas vector search index name |
| `dims` | `u32` | required | Dimensionality of embedding vectors |
| `embedding_field` | `String` | `"embedding"` | Field name for the vector embedding |
| `content_field` | `String` | `"content"` | Field name for document text content |
| `metadata_field` | `String` | `"metadata"` | Field name for document metadata |
