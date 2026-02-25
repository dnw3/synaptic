# Key-Value Store

The `Store` trait provides persistent key-value storage for agents, enabling cross-invocation state management.

## Store Trait

```rust,ignore
use synaptic::store::Store;

#[async_trait]
pub trait Store: Send + Sync {
    async fn get(&self, namespace: &[&str], key: &str) -> Result<Option<Item>, SynapticError>;
    async fn search(&self, namespace: &[&str], query: Option<&str>, limit: usize) -> Result<Vec<Item>, SynapticError>;
    async fn put(&self, namespace: &[&str], key: &str, value: Value) -> Result<(), SynapticError>;
    async fn delete(&self, namespace: &[&str], key: &str) -> Result<(), SynapticError>;
    async fn list_namespaces(&self, prefix: &[&str]) -> Result<Vec<Vec<String>>, SynapticError>;
}
```

Each `Item` returned from `get()` or `search()` contains:

```rust,ignore
pub struct Item {
    pub namespace: Vec<String>,
    pub key: String,
    pub value: Value,
    pub created_at: String,
    pub updated_at: String,
    pub score: Option<f64>,  // populated by semantic search
}
```

## InMemoryStore

```rust,ignore
use synaptic::store::InMemoryStore;

let store = InMemoryStore::new();
store.put(&["users", "prefs"], "theme", json!("dark")).await?;

let item = store.get(&["users", "prefs"], "theme").await?;
```

### Semantic Search

When configured with an embeddings model, `InMemoryStore` uses cosine similarity for `search()` queries instead of substring matching. Items are ranked by relevance and `Item::score` is populated.

```rust,ignore
use synaptic::store::InMemoryStore;
use synaptic::openai::OpenAiEmbeddings;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = InMemoryStore::new().with_embeddings(embeddings);

// Put documents
store.put(&["docs"], "rust", json!("Rust is a systems programming language")).await?;
store.put(&["docs"], "python", json!("Python is an interpreted language")).await?;

// Semantic search — results ranked by similarity
let results = store.search(&["docs"], Some("systems programming"), 10).await?;
// results[0] will be the "rust" item with highest similarity score
assert!(results[0].score.unwrap() > results[1].score.unwrap());
```

Without embeddings, `search()` falls back to substring matching on key and value.

### Hybrid Search (BM25 + Embeddings)

Hybrid search combines BM25 text scoring with embedding similarity using Reciprocal Rank Fusion (RRF). This often outperforms either method alone by capturing both exact keyword matches and semantic similarity.

```rust,ignore
use synaptic::store::InMemoryStore;
use synaptic::openai::OpenAiEmbeddings;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = InMemoryStore::new()
    .with_hybrid_search(embeddings);

store.put(&["docs"], "rust", json!("Rust is a systems programming language focused on safety")).await?;
store.put(&["docs"], "python", json!("Python is great for data science and AI")).await?;

// Hybrid search uses both BM25 term matching and embedding similarity
let results = store.search(&["docs"], Some("safe systems language"), 10).await?;
```

The fusion formula uses `score = Σ 1/(k + rank_i)` with `k=60`, where `rank_i` is the item's rank in each individual result list (BM25 and embedding). This balances exact keyword matches with semantic understanding.

Use `with_embeddings()` for pure vector search, or `with_hybrid_search()` for the combined approach.

## Using with Agents

```rust,ignore
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::store::InMemoryStore;

let store = Arc::new(InMemoryStore::new());
let options = AgentOptions {
    store: Some(store),
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

When a store is provided to `create_agent`, it is automatically wired into `ToolNode`. Any `RuntimeAwareTool` registered with the agent will receive the store via `ToolRuntime`.
