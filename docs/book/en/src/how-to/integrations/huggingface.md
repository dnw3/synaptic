# HuggingFace Embeddings

This crate gives you access to thousands of open-source sentence-transformer models for generating text embeddings via the HuggingFace Inference API.

## Setup

Add the `huggingface` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.2", features = ["huggingface"] }
```

Optionally set your HuggingFace API token:

```bash
export HF_API_KEY="hf_..."
```

## Configuration

```rust,ignore
use synaptic::huggingface::{HuggingFaceEmbeddings, HuggingFaceEmbeddingsConfig};

let config = HuggingFaceEmbeddingsConfig::new("BAAI/bge-small-en-v1.5")
    .with_api_key("hf_...");
let embeddings = HuggingFaceEmbeddings::new(config);
```

## Popular Models

| Model | Dimensions | Use Case |
|-------|------------|----------|
| BAAI/bge-small-en-v1.5 | 384 | Fast English retrieval |
| BAAI/bge-large-en-v1.5 | 1024 | High-quality English retrieval |
| sentence-transformers/all-MiniLM-L6-v2 | 384 | General purpose, popular |
| intfloat/multilingual-e5-large | 1024 | Multilingual retrieval |
| BAAI/bge-m3 | 1024 | Multilingual, long context |

## Usage

### Embed a query

```rust,ignore
use synaptic::core::Embeddings;

let vector = embeddings.embed_query("What is Rust?").await?;
println!("Dimension: {}", vector.len());
```

### Embed documents

```rust,ignore
use synaptic::core::Embeddings;

let docs = ["Rust ensures memory safety", "Python is interpreted"];
let vecs = embeddings.embed_documents(&docs).await?;
```

## RAG Pipeline

Combine HuggingFace embeddings with InMemoryVectorStore for retrieval:

```rust,ignore
use synaptic::huggingface::{HuggingFaceEmbeddings, HuggingFaceEmbeddingsConfig};
use synaptic::vectorstores::InMemoryVectorStore;

let embeddings = std::sync::Arc::new(HuggingFaceEmbeddings::new(
    HuggingFaceEmbeddingsConfig::new("BAAI/bge-small-en-v1.5").with_api_key("hf_..."),
));
let store = std::sync::Arc::new(InMemoryVectorStore::new());
store.add_documents(&docs, embeddings.as_ref()).await?;
let results = retriever.retrieve("memory safe language").await?;
```

## API Key

Get a HuggingFace API token from https://huggingface.co/settings/tokens.
The free tier provides access to public models.
Paid tokens unlock higher rate limits and private model access.

## Configuration Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | String | required | HuggingFace model ID |
| `api_key` | Option | None | API token |
| `base_url` | String | https://api-inference.huggingface.co/models | API base URL |
| `wait_for_model` | bool | true | Wait for model to load |
