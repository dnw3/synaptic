# Buffer Memory

`ConversationBufferMemory` is the simplest memory strategy. It keeps the entire conversation history, returning every message on `load()` with no trimming or summarization.

## Usage

```rust
use std::sync::Arc;
use synapse_memory::{ConversationBufferMemory, InMemoryStore};
use synapse_core::{MemoryStore, Message};

// Create a backing store and wrap it with buffer memory
let store = Arc::new(InMemoryStore::new());
let memory = ConversationBufferMemory::new(store);

let session = "user-1";

memory.append(session, Message::human("Hello")).await?;
memory.append(session, Message::ai("Hi there!")).await?;
memory.append(session, Message::human("What is Rust?")).await?;
memory.append(session, Message::ai("Rust is a systems programming language.")).await?;

let history = memory.load(session).await?;
// Returns ALL 4 messages -- the full conversation
assert_eq!(history.len(), 4);
```

## How It Works

`ConversationBufferMemory` is a thin passthrough wrapper. It delegates `append()`, `load()`, and `clear()` directly to the underlying `MemoryStore` without modification. The "strategy" here is simply: keep everything.

This makes the buffer strategy explicit and composable. By wrapping your store in `ConversationBufferMemory`, you signal that this particular use site intentionally stores full history, and you can later swap in a different strategy (e.g., `ConversationWindowMemory`) without changing the rest of your code.

## When to Use

Buffer memory is a good fit when:

- Conversations are short (under ~20 exchanges) and the full history fits comfortably within the model's context window.
- You need perfect recall of every message (e.g., for auditing or evaluation).
- You are prototyping and do not yet need a more sophisticated strategy.

## Trade-offs

- **Grows unbounded** -- every message is stored and returned. For long conversations, this will eventually exceed the model's context window or cause high token costs.
- **No compression** -- there is no summarization or trimming, so you pay for every token in the history on every LLM call.

If unbounded growth is a concern, consider [Window Memory](window.md) for a fixed-size window, [Token Buffer Memory](token-buffer.md) for a token budget, or [Summary Memory](summary.md) for LLM-based compression.
