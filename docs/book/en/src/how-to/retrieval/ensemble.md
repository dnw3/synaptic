# Ensemble Retriever

This guide shows how to combine multiple retrievers using the `EnsembleRetriever` and Reciprocal Rank Fusion (RRF).

## Overview

Different retrieval strategies have different strengths. Keyword-based methods (like BM25) excel at exact term matching, while vector-based methods capture semantic similarity. The `EnsembleRetriever` combines results from multiple retrievers into a single ranked list, giving you the best of both approaches.

It uses **Reciprocal Rank Fusion (RRF)** to merge rankings. Each retriever contributes a weighted RRF score for each document based on the document's rank in that retriever's results. Documents are then sorted by their total RRF score.

## Basic usage

```rust
use std::sync::Arc;
use synaptic_retrieval::{EnsembleRetriever, Retriever};

let retriever_a: Arc<dyn Retriever> = Arc::new(/* vector retriever */);
let retriever_b: Arc<dyn Retriever> = Arc::new(/* BM25 retriever */);

let ensemble = EnsembleRetriever::new(vec![
    (retriever_a, 0.5),  // weight 0.5
    (retriever_b, 0.5),  // weight 0.5
]);

let results = ensemble.retrieve("query", 5).await?;
```

Each tuple contains a retriever and its weight. The weight scales the RRF score contribution from that retriever.

## Combining vector search with BM25

The most common use case is combining semantic (vector) search with keyword (BM25) search:

```rust
use std::sync::Arc;
use synaptic_retrieval::{BM25Retriever, EnsembleRetriever, Document, Retriever};
use synaptic_vectorstores::{InMemoryVectorStore, VectorStoreRetriever, VectorStore};
use synaptic_embeddings::FakeEmbeddings;

let docs = vec![
    Document::new("1", "Rust provides memory safety through ownership"),
    Document::new("2", "Python has a large ecosystem for machine learning"),
    Document::new("3", "Rust's borrow checker prevents data races"),
    Document::new("4", "Go is designed for building scalable services"),
];

// BM25 retriever (keyword-based)
let bm25 = Arc::new(BM25Retriever::new(docs.clone()));

// Vector retriever (semantic)
let embeddings = Arc::new(FakeEmbeddings::new(128));
let store = Arc::new(InMemoryVectorStore::from_documents(docs, embeddings.as_ref()).await?);
let vector = Arc::new(VectorStoreRetriever::new(store, embeddings, 5));

// Combine with equal weights
let ensemble = EnsembleRetriever::new(vec![
    (vector as Arc<dyn Retriever>, 0.5),
    (bm25 as Arc<dyn Retriever>, 0.5),
]);

let results = ensemble.retrieve("Rust safety", 3).await?;
```

## Adjusting weights

Weights control how much each retriever contributes to the final ranking. Higher weight means more influence.

```rust
// Favor semantic search
let ensemble = EnsembleRetriever::new(vec![
    (vector_retriever, 0.7),
    (bm25_retriever, 0.3),
]);

// Favor keyword search
let ensemble = EnsembleRetriever::new(vec![
    (vector_retriever, 0.3),
    (bm25_retriever, 0.7),
]);
```

## How Reciprocal Rank Fusion works

For each document returned by a retriever, RRF computes a score:

```
rrf_score = weight / (k + rank)
```

Where:
- `weight` is the retriever's configured weight.
- `k` is a constant (60, the standard RRF constant) that prevents top-ranked documents from dominating.
- `rank` is the document's 1-based position in the retriever's results.

If a document appears in results from multiple retrievers, its RRF scores are summed. The final results are sorted by total RRF score in descending order.

## Combining more than two retrievers

You can combine any number of retrievers:

```rust
let ensemble = EnsembleRetriever::new(vec![
    (vector_retriever, 0.4),
    (bm25_retriever, 0.3),
    (multi_query_retriever, 0.3),
]);

let results = ensemble.retrieve("query", 10).await?;
```
