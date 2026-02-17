# Contextual Compression

This guide shows how to post-filter retrieved documents using the `ContextualCompressionRetriever` and `EmbeddingsFilter`.

## Overview

A base retriever may return documents that are only loosely related to the query. Contextual compression adds a second filtering step: after retrieval, a `DocumentCompressor` evaluates each document against the query and removes documents that do not meet a relevance threshold.

This is especially useful when your base retriever fetches broadly (high recall) and you want to tighten the results (high precision).

## DocumentCompressor trait

The filtering logic is defined by the `DocumentCompressor` trait:

```rust
#[async_trait]
pub trait DocumentCompressor: Send + Sync {
    async fn compress_documents(
        &self,
        documents: Vec<Document>,
        query: &str,
    ) -> Result<Vec<Document>, SynapticError>;
}
```

Synaptic provides `EmbeddingsFilter` as a built-in compressor.

## EmbeddingsFilter

Filters documents by computing cosine similarity between the query embedding and each document's content embedding. Only documents that meet or exceed the similarity threshold are kept.

```rust
use std::sync::Arc;
use synaptic_retrieval::EmbeddingsFilter;
use synaptic_embeddings::FakeEmbeddings;

let embeddings = Arc::new(FakeEmbeddings::new(128));

// Only keep documents with similarity >= 0.7
let filter = EmbeddingsFilter::new(embeddings, 0.7);
```

A convenience constructor uses the default threshold of `0.75`:

```rust
let filter = EmbeddingsFilter::with_default_threshold(embeddings);
```

## ContextualCompressionRetriever

Wraps a base retriever and applies a `DocumentCompressor` to the results:

```rust
use std::sync::Arc;
use synaptic_retrieval::{
    ContextualCompressionRetriever,
    EmbeddingsFilter,
    Retriever,
};
use synaptic_embeddings::FakeEmbeddings;

let embeddings = Arc::new(FakeEmbeddings::new(128));
let base_retriever: Arc<dyn Retriever> = Arc::new(/* any retriever */);

// Create the filter
let filter = Arc::new(EmbeddingsFilter::new(embeddings, 0.7));

// Wrap the base retriever with compression
let retriever = ContextualCompressionRetriever::new(base_retriever, filter);

let results = retriever.retrieve("query", 5).await?;
// Only documents with cosine similarity >= 0.7 to the query are returned
```

## Full example

```rust
use std::sync::Arc;
use synaptic_retrieval::{
    BM25Retriever,
    ContextualCompressionRetriever,
    EmbeddingsFilter,
    Document,
    Retriever,
};
use synaptic_embeddings::FakeEmbeddings;

let docs = vec![
    Document::new("1", "Rust is a systems programming language"),
    Document::new("2", "The weather today is sunny and warm"),
    Document::new("3", "Rust provides memory safety guarantees"),
    Document::new("4", "Cooking pasta requires boiling water"),
];

// BM25 might return loosely relevant results
let base = Arc::new(BM25Retriever::new(docs));

// Use embedding similarity to filter out irrelevant documents
let embeddings = Arc::new(FakeEmbeddings::new(128));
let filter = Arc::new(EmbeddingsFilter::new(embeddings, 0.6));
let retriever = ContextualCompressionRetriever::new(base, filter);

let results = retriever.retrieve("Rust programming", 5).await?;
// Documents about weather and cooking are filtered out
```

## How it works

1. The `ContextualCompressionRetriever` calls `base.retrieve(query, top_k)` to get candidate documents.
2. It passes those candidates to the `DocumentCompressor` (e.g., `EmbeddingsFilter`).
3. The compressor embeds the query and all candidate documents, computes cosine similarity, and removes documents below the threshold.
4. The filtered results are returned.

## Custom compressors

You can implement your own `DocumentCompressor` for other filtering strategies -- for example, using an LLM to judge relevance or extracting only the most relevant passage from each document.

```rust
use async_trait::async_trait;
use synaptic_retrieval::{DocumentCompressor, Document};
use synaptic_core::SynapticError;

struct MyCompressor;

#[async_trait]
impl DocumentCompressor for MyCompressor {
    async fn compress_documents(
        &self,
        documents: Vec<Document>,
        query: &str,
    ) -> Result<Vec<Document>, SynapticError> {
        // Your filtering logic here
        Ok(documents)
    }
}
```
