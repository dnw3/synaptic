# Retrieval

Synapse provides a complete Retrieval-Augmented Generation (RAG) pipeline. The pipeline follows five stages:

1. **Load** -- ingest raw data from files, JSON, CSV, web URLs, or entire directories.
2. **Split** -- break large documents into smaller chunks that fit within context windows.
3. **Embed** -- convert text chunks into numerical vectors using an embedding model.
4. **Store** -- persist embeddings in a vector store for efficient similarity search.
5. **Retrieve** -- find the most relevant documents for a given query.

## Key types

| Type | Crate | Purpose |
|------|-------|---------|
| `Document` | `synapse_retrieval` | A unit of text with `id`, `content`, and `metadata: HashMap<String, Value>` |
| `Loader` trait | `synapse_loaders` | Async trait for loading documents from various sources |
| `TextSplitter` trait | `synapse_splitters` | Splits text into chunks with optional overlap |
| `Embeddings` trait | `synapse_embeddings` | Converts text into vector representations |
| `VectorStore` trait | `synapse_vectorstores` | Stores and searches document embeddings |
| `Retriever` trait | `synapse_retrieval` | Retrieves relevant documents given a query string |

## Retrievers

Synapse ships with seven retriever implementations, each suited to different use cases:

| Retriever | Strategy |
|-----------|----------|
| `VectorStoreRetriever` | Wraps any `VectorStore` for cosine similarity search |
| `BM25Retriever` | Okapi BM25 keyword scoring -- no embeddings required |
| `MultiQueryRetriever` | Uses an LLM to generate query variants, retrieves for each, deduplicates |
| `EnsembleRetriever` | Combines multiple retrievers via Reciprocal Rank Fusion |
| `ContextualCompressionRetriever` | Post-filters retrieved documents using a `DocumentCompressor` |
| `SelfQueryRetriever` | Uses an LLM to extract structured metadata filters from natural language |
| `ParentDocumentRetriever` | Searches small child chunks but returns full parent documents |

## Guides

- [Document Loaders](loaders.md) -- load data from text, JSON, CSV, files, directories, and the web
- [Text Splitters](splitters.md) -- break documents into chunks with character, recursive, markdown, or token-based strategies
- [Embeddings](embeddings.md) -- embed text using OpenAI, Ollama, or deterministic fake embeddings
- [Vector Stores](vector-stores.md) -- store and search embeddings with `InMemoryVectorStore`
- [BM25 Retriever](bm25.md) -- keyword-based retrieval with Okapi BM25 scoring
- [Multi-Query Retriever](multi-query.md) -- improve recall by generating multiple query perspectives
- [Ensemble Retriever](ensemble.md) -- combine retrievers with Reciprocal Rank Fusion
- [Contextual Compression](compression.md) -- post-filter results with embedding similarity thresholds
- [Self-Query Retriever](self-query.md) -- LLM-powered metadata filtering from natural language
- [Parent Document Retriever](parent-document.md) -- search small chunks, return full parent documents
