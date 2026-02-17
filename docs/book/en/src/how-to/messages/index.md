# Messages

Messages are the fundamental unit of communication in Synaptic. Every interaction with a chat model is expressed as a sequence of `Message` values, and every response comes back as a `Message`.

The `Message` enum is defined in `synaptic_core` and uses a tagged union with six variants: `System`, `Human`, `AI`, `Tool`, `Chat`, and `Remove`. You create messages through factory methods rather than struct literals.

## Quick example

```rust
use synaptic_core::{ChatRequest, Message};

let messages = vec![
    Message::system("You are a helpful assistant."),
    Message::human("What is Rust?"),
];

let request = ChatRequest::new(messages);
```

## Guides

- [Message Types](types.md) -- all message variants, factory methods, and accessor methods
- [Filter & Trim Messages](filter-trim.md) -- select messages by type/name/id and trim to a token budget
- [Merge Message Runs](merge-runs.md) -- combine consecutive messages of the same role into one
