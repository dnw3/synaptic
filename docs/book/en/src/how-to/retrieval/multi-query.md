# Multi-Query Retriever

This guide shows how to use the `MultiQueryRetriever` to improve retrieval recall by generating multiple query perspectives with an LLM.

## Overview

A single search query may not capture all relevant documents, especially when the user's phrasing does not match the vocabulary in the document corpus. The `MultiQueryRetriever` addresses this by:

1. Using a `ChatModel` to generate alternative phrasings of the original query.
2. Running each query variant through a base retriever.
3. Deduplicating and merging the results.

This technique improves recall by overcoming limitations of distance-based similarity search.

## Basic usage

```rust
use std::sync::Arc;
use synapse_retrieval::{MultiQueryRetriever, Retriever};

let base_retriever: Arc<dyn Retriever> = Arc::new(/* any retriever */);
let model: Arc<dyn ChatModel> = Arc::new(/* any ChatModel */);

// Default: generates 3 query variants
let retriever = MultiQueryRetriever::new(base_retriever, model);

let results = retriever.retrieve("What are the benefits of Rust?", 5).await?;
```

When you call `retrieve()`, the retriever:

1. Sends a prompt to the LLM asking it to rephrase the query into 3 different versions.
2. Runs the original query plus all generated variants through the base retriever.
3. Collects all results, deduplicates by document `id`, and returns up to `top_k` documents.

## Custom number of query variants

Specify a different number of generated queries:

```rust
let retriever = MultiQueryRetriever::with_num_queries(
    base_retriever,
    model,
    5,  // Generate 5 query variants
);
```

More variants increase recall but also increase the number of LLM and retriever calls.

## How it works internally

The retriever sends a prompt like this to the LLM:

> You are an AI language model assistant. Your task is to generate 3 different versions of the given user question to retrieve relevant documents from a vector database. By generating multiple perspectives on the user question, your goal is to help the user overcome some of the limitations of distance-based similarity search. Provide these alternative questions separated by newlines. Only output the questions, nothing else.
>
> Original question: What are the benefits of Rust?

The LLM might respond with:

```
Why should I use Rust as a programming language?
What advantages does Rust offer over other languages?
What makes Rust a good choice for software development?
```

Each of these queries is then run through the base retriever, and all results are merged with deduplication.

## Example with a vector store

```rust
use std::sync::Arc;
use synapse_retrieval::{MultiQueryRetriever, Retriever};
use synapse_vectorstores::{InMemoryVectorStore, VectorStoreRetriever, VectorStore};
use synapse_embeddings::FakeEmbeddings;
use synapse_retrieval::Document;

// Set up vector store
let embeddings = Arc::new(FakeEmbeddings::new(128));
let store = Arc::new(InMemoryVectorStore::new());

let docs = vec![
    Document::new("1", "Rust ensures memory safety without a garbage collector"),
    Document::new("2", "Rust's ownership system prevents data races at compile time"),
    Document::new("3", "Go uses goroutines for lightweight concurrency"),
];
store.add_documents(docs, embeddings.as_ref()).await?;

// Wrap vector store as a retriever
let base = Arc::new(VectorStoreRetriever::new(store, embeddings, 5));

// Create multi-query retriever
let model: Arc<dyn ChatModel> = Arc::new(/* your model */);
let retriever = MultiQueryRetriever::new(base, model);

let results = retriever.retrieve("Why is Rust safe?", 5).await?;
// May find documents that mention "memory safety", "ownership", "data races"
// even if the original query doesn't use those exact terms
```
