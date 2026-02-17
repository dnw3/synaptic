# Window Memory

`ConversationWindowMemory` keeps only the most recent K messages. All messages are stored in the underlying store, but `load()` returns a sliding window of the last `window_size` messages.

## Usage

```rust
use std::sync::Arc;
use synaptic_memory::{ConversationWindowMemory, InMemoryStore};
use synaptic_core::{MemoryStore, Message};

let store = Arc::new(InMemoryStore::new());

// Keep only the last 4 messages visible
let memory = ConversationWindowMemory::new(store, 4);

let session = "user-1";

memory.append(session, Message::human("Message 1")).await?;
memory.append(session, Message::ai("Reply 1")).await?;
memory.append(session, Message::human("Message 2")).await?;
memory.append(session, Message::ai("Reply 2")).await?;
memory.append(session, Message::human("Message 3")).await?;
memory.append(session, Message::ai("Reply 3")).await?;

let history = memory.load(session).await?;
// Only the last 4 messages are returned
assert_eq!(history.len(), 4);
assert_eq!(history[0].content(), "Message 2");
assert_eq!(history[3].content(), "Reply 3");
```

## How It Works

- **`append()`** stores every message in the underlying `MemoryStore` -- nothing is discarded on write.
- **`load()`** retrieves all messages from the store, then returns only the last `window_size` entries. If the total number of messages is less than or equal to `window_size`, all messages are returned.
- **`clear()`** removes all messages from the underlying store for the given session.

The window is applied at load time, not at write time. This means the full history remains in the backing store and could be accessed directly if needed.

## Choosing `window_size`

The `window_size` parameter is measured in individual messages, not pairs. A typical human/AI exchange produces 2 messages, so a `window_size` of 10 keeps roughly 5 turns of conversation.

Consider your model's context window when choosing a size. A window of 20 messages is usually safe for most models, while a window of 4-6 messages works well for lightweight chat UIs where only the most recent context matters.

## When to Use

Window memory is a good fit when:

- You want fixed, predictable memory usage with no LLM calls for summarization.
- Older context is genuinely less relevant (e.g., a casual chatbot or customer support flow).
- You need a simple strategy that is easy to reason about.

## Trade-offs

- **Hard cutoff** -- messages outside the window are invisible to the model. There is no summary or compressed representation of older history.
- **No token awareness** -- the window is measured in message count, not token count. A few long messages could still exceed the model's context window. If you need token-level control, see [Token Buffer Memory](token-buffer.md).

For a strategy that preserves older context through summarization, see [Summary Memory](summary.md) or [Summary Buffer Memory](summary-buffer.md).
