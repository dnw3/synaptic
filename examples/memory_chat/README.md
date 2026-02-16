# memory_chat

Demonstrates session-based conversation memory using `InMemoryStore`.

## What it does

1. Creates an `InMemoryStore` for conversation storage
2. Appends human and AI messages to a session
3. Loads the full transcript and prints each message with its role

## Run

```bash
cargo run -p memory_chat
```

## Expected output

```
human: Hello, Synapse
ai: Hello, how can I help you?
```

## Key concepts

- **`MemoryStore` trait** — `append(session_id, message)` and `load(session_id)` for session-keyed storage
- **`InMemoryStore`** — in-memory implementation backed by `HashMap<String, Vec<Message>>`
- **`Message` factory methods** — `Message::human()`, `Message::ai()` create typed messages
- **Session isolation** — each `session_id` maintains its own independent message history
