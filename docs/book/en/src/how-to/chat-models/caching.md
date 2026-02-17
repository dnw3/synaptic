# Caching LLM Responses

This guide shows how to cache LLM responses to avoid redundant API calls and reduce latency.

## Overview

Synapse provides two cache implementations through the `LlmCache` trait:

- **`InMemoryCache`** -- exact-match caching with optional TTL expiration.
- **`SemanticCache`** -- embedding-based similarity matching for semantically equivalent queries.

Both are used with `CachedChatModel`, which wraps any `ChatModel` and checks the cache before making an API call.

## Exact-match caching with `InMemoryCache`

The simplest cache stores responses keyed by the exact request content:

```rust
use std::sync::Arc;
use synaptic_core::ChatModel;
use synaptic_cache::{InMemoryCache, CachedChatModel};

let base_model: Arc<dyn ChatModel> = Arc::new(model);
let cache = Arc::new(InMemoryCache::new());
let cached_model = CachedChatModel::new(base_model, cache);

// First call hits the LLM
// let response1 = cached_model.chat(request.clone()).await?;

// Identical request returns cached response instantly
// let response2 = cached_model.chat(request.clone()).await?;
```

### Cache with TTL

Set a time-to-live so entries expire automatically:

```rust
use std::time::Duration;
use std::sync::Arc;
use synaptic_cache::InMemoryCache;

// Entries expire after 1 hour
let cache = Arc::new(InMemoryCache::with_ttl(Duration::from_secs(3600)));

// Entries expire after 5 minutes
let cache = Arc::new(InMemoryCache::with_ttl(Duration::from_secs(300)));
```

After the TTL elapses, a cache lookup for that entry returns `None`, and the next request will hit the LLM again.

## Semantic caching with `SemanticCache`

Semantic caching uses embeddings to find similar queries, even when the exact wording differs. For example, "What's the weather?" and "Tell me the current weather" could match the same cached response.

```rust
use std::sync::Arc;
use synaptic_cache::{SemanticCache, CachedChatModel};
use synaptic_embeddings::OpenAiEmbeddings;

let embeddings: Arc<dyn synaptic_embeddings::Embeddings> = Arc::new(embeddings_provider);

// Similarity threshold of 0.95 means only very similar queries match
let cache = Arc::new(SemanticCache::new(embeddings, 0.95));

let cached_model = CachedChatModel::new(base_model, cache);
```

When looking up a cached response:

1. The query is embedded using the provided `Embeddings` implementation.
2. The embedding is compared against all stored entries using cosine similarity.
3. If the best match exceeds the similarity threshold, the cached response is returned.

### Choosing a threshold

- **0.95 -- 0.99**: Very strict. Only nearly identical queries match. Good for factual Q&A where slight wording changes can change meaning.
- **0.90 -- 0.95**: Moderate. Catches common rephrasing. Good for general-purpose chatbots.
- **0.80 -- 0.90**: Loose. Broader matching. Useful when you want aggressive caching and approximate answers are acceptable.

## The `LlmCache` trait

Both cache types implement the `LlmCache` trait:

```rust
#[async_trait]
pub trait LlmCache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<ChatResponse>, SynapseError>;
    async fn put(&self, key: &str, response: &ChatResponse) -> Result<(), SynapseError>;
    async fn clear(&self) -> Result<(), SynapseError>;
}
```

You can implement this trait for custom cache backends (Redis, SQLite, etc.).

## Clearing the cache

Both cache implementations support clearing all entries:

```rust
use synaptic_cache::LlmCache;

// cache implements LlmCache
// cache.clear().await?;
```

## Combining with other wrappers

Since `CachedChatModel` implements `ChatModel`, it composes with retry, rate limiting, and other wrappers:

```rust
use std::sync::Arc;
use synaptic_core::ChatModel;
use synaptic_cache::{InMemoryCache, CachedChatModel};
use synaptic_models::{RetryChatModel, RetryPolicy};

let base_model: Arc<dyn ChatModel> = Arc::new(model);

// Cache first, then retry on cache miss + API failure
let cache = Arc::new(InMemoryCache::new());
let cached = Arc::new(CachedChatModel::new(base_model, cache));
let reliable = RetryChatModel::new(cached, RetryPolicy::default());
```
