# Parent Document Retriever

This guide shows how to use the `ParentDocumentRetriever` to search on small chunks for precision while returning full parent documents for context.

## The problem

When splitting documents for retrieval, you face a trade-off:

- **Small chunks** are better for search precision -- they match queries more accurately because there is less noise.
- **Large documents** are better for context -- they give the LLM more information to work with when generating answers.

The `ParentDocumentRetriever` solves this by maintaining both: it splits parent documents into small child chunks for indexing, but when a child chunk matches a query, it returns the full parent document.

## How it works

1. You provide parent documents and a splitting function.
2. The retriever splits each parent into child chunks, storing a child-to-parent mapping.
3. Child chunks are indexed in a child retriever (e.g., backed by a vector store).
4. At retrieval time, the child retriever finds matching chunks, then the parent retriever maps those back to their parent documents, deduplicating along the way.

## Basic usage

```rust
use std::sync::Arc;
use synapse_retrieval::{ParentDocumentRetriever, Document, Retriever};
use synapse_splitters::{RecursiveCharacterTextSplitter, TextSplitter};

// Create a child retriever (any Retriever implementation)
let child_retriever: Arc<dyn Retriever> = Arc::new(/* vector store retriever */);

// Create the parent document retriever with a splitting function
let splitter = RecursiveCharacterTextSplitter::new(200);
let parent_retriever = ParentDocumentRetriever::new(
    child_retriever.clone(),
    move |text: &str| splitter.split_text(text),
);
```

## Adding documents

The `add_documents()` method splits parent documents into children and stores the mappings. It returns the child documents so you can index them in the child retriever.

```rust
let parent_docs = vec![
    Document::new("doc-1", "A very long document about Rust ownership..."),
    Document::new("doc-2", "A detailed guide to async programming in Rust..."),
];

// Split parents into children and get child docs for indexing
let child_docs = parent_retriever.add_documents(parent_docs).await;

// Index child docs in the vector store
// child_docs[0].id == "doc-1-child-0"
// child_docs[0].metadata["parent_id"] == "doc-1"
// child_docs[0].metadata["chunk_index"] == 0
```

Each child document:
- Has an ID formatted as `"{parent_id}-child-{index}"`.
- Inherits all metadata from the parent.
- Gets additional `parent_id` and `chunk_index` metadata fields.

## Retrieval

When you call `retrieve()`, the retriever searches for matching child chunks, then returns the corresponding parent documents:

```rust
let results = parent_retriever.retrieve("ownership borrowing", 3).await?;
// Returns full parent documents, not individual chunks
```

The retriever fetches `top_k * 3` child results internally to ensure enough parent documents can be assembled after deduplication.

## Full example

```rust
use std::sync::Arc;
use synapse_retrieval::{ParentDocumentRetriever, Document, Retriever};
use synapse_vectorstores::{InMemoryVectorStore, VectorStoreRetriever, VectorStore};
use synapse_embeddings::FakeEmbeddings;
use synapse_splitters::{RecursiveCharacterTextSplitter, TextSplitter};

// Set up embeddings and vector store for child chunks
let embeddings = Arc::new(FakeEmbeddings::new(128));
let child_store = Arc::new(InMemoryVectorStore::new());

// Create the child retriever
let child_retriever = Arc::new(VectorStoreRetriever::new(
    child_store.clone(),
    embeddings.clone(),
    10,
));

// Create parent retriever with a small chunk size for children
let splitter = RecursiveCharacterTextSplitter::new(200);
let parent_retriever = ParentDocumentRetriever::new(
    child_retriever,
    move |text: &str| splitter.split_text(text),
);

// Add parent documents
let parents = vec![
    Document::new("rust-guide", "A comprehensive guide to Rust. \
        Rust is a systems programming language focused on safety, speed, and concurrency. \
        It achieves memory safety without garbage collection through its ownership system. \
        The borrow checker enforces ownership rules at compile time..."),
    Document::new("go-guide", "A comprehensive guide to Go. \
        Go is a statically typed language designed at Google. \
        It features goroutines for lightweight concurrency. \
        Go's garbage collector manages memory automatically..."),
];

let children = parent_retriever.add_documents(parents).await;

// Index children in the vector store
child_store.add_documents(children, embeddings.as_ref()).await?;

// Search for child chunks, get back full parent documents
let results = parent_retriever.retrieve("memory safety ownership", 2).await?;
// Returns the full "rust-guide" parent document, even though only
// a small chunk about ownership matched the query
```

## When to use this

The `ParentDocumentRetriever` is most useful when:

- Your documents are long and cover multiple topics, but you want precise retrieval.
- You need the LLM to see the full document context for generating high-quality answers.
- Small chunks alone would lose important surrounding context.

For simpler use cases where chunks are self-contained, a standard `VectorStoreRetriever` may be sufficient.
