# Graph Checkpointers

By default, `synaptic-graph` uses [`MemorySaver`], which stores graph state only in-process memory. This means state is lost when the process restarts — not suitable for production.

Synaptic provides two persistent checkpointer backends:

| Backend | Crate | Feature flag |
|---------|-------|--------------|
| Redis | `synaptic-redis` | `redis` + `checkpointer` |
| PostgreSQL | `synaptic-pgvector` | `pgvector` + `checkpointer` |

## Setup

Add the relevant feature flags to `Cargo.toml`:

```toml
# Redis checkpointer
[dependencies]
synaptic = { version = "0.2", features = ["agent", "redis"] }
synaptic-redis = { version = "0.2", features = ["checkpointer"] }

# Or PostgreSQL checkpointer
synaptic = { version = "0.2", features = ["agent", "pgvector"] }
synaptic-pgvector = { version = "0.2", features = ["checkpointer"] }
```

## Redis Checkpointer

### Quick start

```rust,ignore
use synaptic_redis::checkpointer::{RedisCheckpointer, RedisCheckpointerConfig};
use synaptic::graph::{create_react_agent, MessageState};
use std::sync::Arc;

// Connect to Redis
let checkpointer = RedisCheckpointer::from_url("redis://127.0.0.1/").await?;

// Build the graph with the persistent checkpointer
let graph = create_react_agent(model, tools)?
    .with_checkpointer(Arc::new(checkpointer));

// Run with a thread ID for persistence
let state = MessageState { messages: vec![Message::human("Hello")] };
let config = RunnableConfig::default().with_metadata("thread_id", "user-123");
let result = graph.invoke_with_config(state, config).await?;
```

### Configuration

```rust,ignore
use synaptic_redis::checkpointer::RedisCheckpointerConfig;

let config = RedisCheckpointerConfig::new("redis://127.0.0.1/")
    .with_ttl(86400)          // Expire checkpoints after 24 hours
    .with_prefix("myapp");    // Custom key prefix (default: "synaptic")

let checkpointer = RedisCheckpointer::new(config).await?;
```

### Configuration reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | required | Redis connection URL |
| `ttl` | `Option<u64>` | `None` | TTL in seconds for checkpoint keys |
| `prefix` | `String` | `"synaptic"` | Key prefix for all checkpoint keys |

### Key scheme

Redis stores checkpoints using the following keys:

- **Checkpoint data**: `{prefix}:checkpoint:{thread_id}:{checkpoint_id}` — JSON-serialized `Checkpoint`
- **Thread index**: `{prefix}:idx:{thread_id}` — Redis LIST of checkpoint IDs in chronological order

## PostgreSQL Checkpointer

### Quick start

```rust,ignore
use sqlx::postgres::PgPoolOptions;
use synaptic_pgvector::checkpointer::PgCheckpointer;
use synaptic::graph::{create_react_agent, MessageState};
use std::sync::Arc;

// Create a connection pool
let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect("postgres://user:pass@localhost/mydb")
    .await?;

// Create and initialize the checkpointer (creates table if not exists)
let checkpointer = PgCheckpointer::new(pool);
checkpointer.initialize().await?;

// Build the graph
let graph = create_react_agent(model, tools)?
    .with_checkpointer(Arc::new(checkpointer));
```

### Schema

`initialize()` creates the following table if it does not exist:

```sql
CREATE TABLE IF NOT EXISTS synaptic_checkpoints (
    thread_id     TEXT        NOT NULL,
    checkpoint_id TEXT        NOT NULL,
    state         JSONB       NOT NULL,
    next_node     TEXT,
    parent_id     TEXT,
    metadata      JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (thread_id, checkpoint_id)
);
```

### Custom table name

```rust,ignore
let checkpointer = PgCheckpointer::new(pool)
    .with_table("my_custom_checkpoints");
checkpointer.initialize().await?;
```

## Human-in-the-loop with persistence

Persistent checkpointers enable stateful human-in-the-loop workflows:

```rust,ignore
use synaptic::graph::{StateGraph, MessageState, StreamMode};
use synaptic_redis::checkpointer::RedisCheckpointer;

let checkpointer = Arc::new(RedisCheckpointer::from_url("redis://localhost/").await?);

// Compile graph with interrupt before "human_review" node
let graph = builder
    .interrupt_before(vec!["human_review"])
    .compile_with_checkpointer(checkpointer)?;

// First invocation — graph pauses before "human_review"
let config = RunnableConfig::default().with_metadata("thread_id", "session-42");
let result = graph.invoke_with_config(initial_state, config.clone()).await?;

// Inject human feedback and resume
let updated = graph.update_state(config.clone(), feedback_state).await?;
let final_result = graph.invoke_with_config(updated, config).await?;
```

## Time-travel debugging

Retrieve any historical checkpoint by ID for debugging or replaying:

```rust,ignore
use synaptic_graph::checkpoint::{CheckpointConfig, Checkpointer};

let config = CheckpointConfig::with_checkpoint_id("thread-123", "specific-checkpoint-id");
if let Some(checkpoint) = checkpointer.get(&config).await? {
    println!("State at checkpoint: {:?}", checkpoint.state);
}

// List all checkpoints for a thread
let all = checkpointer.list(&CheckpointConfig::new("thread-123")).await?;
println!("Total checkpoints: {}", all.len());
```

## Error handling

```rust,ignore
use synaptic::core::SynapticError;

match checkpointer.get(&config).await {
    Ok(Some(cp)) => println!("Loaded checkpoint: {}", cp.id),
    Ok(None) => println!("No checkpoint found"),
    Err(SynapticError::Store(msg)) => eprintln!("Storage error: {msg}"),
    Err(e) => return Err(e.into()),
}
```
