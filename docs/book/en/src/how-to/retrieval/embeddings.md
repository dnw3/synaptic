# Embeddings

This guide shows how to convert text into vector representations using Synapse's `Embeddings` trait and its built-in providers.

## Overview

All embedding providers implement the `Embeddings` trait from `synapse_embeddings`:

```rust
#[async_trait]
pub trait Embeddings: Send + Sync {
    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SynapseError>;
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, SynapseError>;
}
```

- `embed_documents()` embeds multiple texts in a single batch -- use this for indexing.
- `embed_query()` embeds a single query text -- use this at retrieval time.

## FakeEmbeddings

Generates deterministic vectors based on a simple hash of the input text. Useful for testing and development without API calls.

```rust
use synapse_embeddings::FakeEmbeddings;
use synapse_embeddings::Embeddings;

// Specify the number of dimensions (default is 4)
let embeddings = FakeEmbeddings::new(4);

let doc_vectors = embeddings.embed_documents(&["doc one", "doc two"]).await?;
let query_vector = embeddings.embed_query("search query").await?;

// Vectors are normalized to unit length
// Similar texts produce similar vectors
```

## OpenAiEmbeddings

Uses the OpenAI embeddings API. Requires an API key and a `ProviderBackend`.

```rust
use std::sync::Arc;
use synapse_embeddings::{OpenAiEmbeddings, OpenAiEmbeddingsConfig};
use synapse_embeddings::Embeddings;
use synapse_models::backend::HttpBackend;

let config = OpenAiEmbeddingsConfig::new("sk-...")
    .with_model("text-embedding-3-small");  // default model

let backend = Arc::new(HttpBackend::new());
let embeddings = OpenAiEmbeddings::new(config, backend);

let vectors = embeddings.embed_documents(&["hello world"]).await?;
```

You can customize the base URL for compatible APIs:

```rust
let config = OpenAiEmbeddingsConfig::new("sk-...")
    .with_base_url("https://my-proxy.example.com/v1");
```

## OllamaEmbeddings

Uses a local Ollama instance for embedding. No API key required -- just specify the model name.

```rust
use std::sync::Arc;
use synapse_embeddings::{OllamaEmbeddings, OllamaEmbeddingsConfig};
use synapse_embeddings::Embeddings;
use synapse_models::backend::HttpBackend;

let config = OllamaEmbeddingsConfig::new("nomic-embed-text");
// Default base_url: http://localhost:11434

let backend = Arc::new(HttpBackend::new());
let embeddings = OllamaEmbeddings::new(config, backend);

let vector = embeddings.embed_query("search query").await?;
```

Custom Ollama endpoint:

```rust
let config = OllamaEmbeddingsConfig::new("nomic-embed-text")
    .with_base_url("http://my-ollama:11434");
```

## CacheBackedEmbeddings

Wraps any `Embeddings` provider with an in-memory cache. Previously computed embeddings are returned from cache; only uncached texts are sent to the underlying provider.

```rust
use std::sync::Arc;
use synapse_embeddings::{CacheBackedEmbeddings, FakeEmbeddings, Embeddings};

let inner = Arc::new(FakeEmbeddings::new(128));
let cached = CacheBackedEmbeddings::new(inner);

// First call computes the embedding
let v1 = cached.embed_query("hello").await?;

// Second call returns the cached result -- no recomputation
let v2 = cached.embed_query("hello").await?;

assert_eq!(v1, v2);
```

This is especially useful when adding documents to a vector store and then querying, since the same text may be embedded multiple times across operations.

## Using embeddings with vector stores

Embeddings are passed to vector store methods rather than stored inside the vector store. This lets you swap embedding providers without rebuilding the store.

```rust
use synapse_vectorstores::{InMemoryVectorStore, VectorStore};
use synapse_embeddings::FakeEmbeddings;
use synapse_retrieval::Document;

let embeddings = FakeEmbeddings::new(128);
let store = InMemoryVectorStore::new();

let docs = vec![Document::new("1", "Rust is fast")];
store.add_documents(docs, &embeddings).await?;

let results = store.similarity_search("fast language", 5, &embeddings).await?;
```
