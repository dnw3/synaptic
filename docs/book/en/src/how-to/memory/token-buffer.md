# Token Buffer Memory

`ConversationTokenBufferMemory` keeps the most recent messages that fit within a token budget. On `load()`, the oldest messages are dropped until the total estimated token count is at or below `max_tokens`.

## Usage

```rust
use std::sync::Arc;
use synapse_memory::{ConversationTokenBufferMemory, InMemoryStore};
use synapse_core::{MemoryStore, Message};

let store = Arc::new(InMemoryStore::new());

// Keep messages within a 200-token budget
let memory = ConversationTokenBufferMemory::new(store, 200);

let session = "user-1";

memory.append(session, Message::human("Hello!")).await?;
memory.append(session, Message::ai("Hi! How can I help?")).await?;
memory.append(session, Message::human("Tell me a long story about Rust.")).await?;
memory.append(session, Message::ai("Rust began as a personal project...")).await?;

let history = memory.load(session).await?;
// Only messages that fit within 200 estimated tokens are returned.
// Oldest messages are dropped first.
```

## How It Works

- **`append()`** stores every message in the underlying `MemoryStore` without modification.
- **`load()`** retrieves all messages, estimates their total token count, and removes the oldest messages one by one until the total fits within `max_tokens`.
- **`clear()`** removes all messages from the underlying store for the session.

### Token Estimation

Synapse uses a simple heuristic of approximately 4 characters per token, with a minimum of 1 token per message:

```rust
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4 + 1
}
```

This is a rough approximation. Actual token counts vary by model and tokenizer. The heuristic is intentionally conservative (slightly overestimates) to avoid exceeding real token limits.

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `store` | `Arc<dyn MemoryStore>` | The backing store for raw messages |
| `max_tokens` | `usize` | Maximum estimated tokens to return from `load()` |

## When to Use

Token buffer memory is a good fit when:

- You need to control prompt size in token terms rather than message count.
- You want to stay within a model's context window without manually counting messages.
- You prefer a simple, no-LLM-call strategy for managing memory size.

## Trade-offs

- **Approximate** -- the token estimate is a heuristic, not an exact count. For precise token budgeting, you would need a model-specific tokenizer.
- **Hard cutoff** -- dropped messages are lost entirely. There is no summary or compressed representation of older history.
- **Drops whole messages** -- if a single message is very long, it may consume most of the budget by itself.

For a fixed message count instead of a token budget, see [Window Memory](window.md). For a strategy that preserves older context through summarization, see [Summary Memory](summary.md) or [Summary Buffer Memory](summary-buffer.md).
