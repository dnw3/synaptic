# Filter & Trim Messages

This guide shows how to select specific messages from a conversation and trim message lists to fit within token budgets.

## Filtering messages with `filter_messages`

The `filter_messages` function selects messages based on their type (role), name, or ID. It supports both inclusion and exclusion filters.

```rust
use synaptic::core::{filter_messages, Message};
```

### Filter by type

```rust
let messages = vec![
    Message::system("You are helpful."),
    Message::human("Question 1"),
    Message::ai("Answer 1"),
    Message::human("Question 2"),
    Message::ai("Answer 2"),
];

// Keep only human messages
let humans = filter_messages(
    &messages,
    Some(&["human"]),  // include_types
    None,              // exclude_types
    None,              // include_names
    None,              // exclude_names
    None,              // include_ids
    None,              // exclude_ids
);
assert_eq!(humans.len(), 2);
assert_eq!(humans[0].content(), "Question 1");
assert_eq!(humans[1].content(), "Question 2");
```

### Exclude by type

```rust
// Remove system messages, keep everything else
let without_system = filter_messages(
    &messages,
    None,                // include_types
    Some(&["system"]),   // exclude_types
    None, None, None, None,
);
assert_eq!(without_system.len(), 4);
```

### Filter by name

```rust
let messages = vec![
    Message::human("Hi").with_name("Alice"),
    Message::human("Hello").with_name("Bob"),
    Message::ai("Hey!"),
];

// Only messages from Alice
let alice_msgs = filter_messages(
    &messages,
    None, None,
    Some(&["Alice"]),  // include_names
    None, None, None,
);
assert_eq!(alice_msgs.len(), 1);
assert_eq!(alice_msgs[0].content(), "Hi");
```

### Filter by ID

```rust
let messages = vec![
    Message::human("First").with_id("msg-1"),
    Message::human("Second").with_id("msg-2"),
    Message::human("Third").with_id("msg-3"),
];

// Exclude a specific message
let filtered = filter_messages(
    &messages,
    None, None, None, None,
    None,                         // include_ids
    Some(&["msg-2"]),             // exclude_ids
);
assert_eq!(filtered.len(), 2);
```

### Combining filters

All filter parameters can be combined. A message must pass all active filters to be included:

```rust
// Keep only human messages from Alice
let result = filter_messages(
    &messages,
    Some(&["human"]),    // include_types
    None,                // exclude_types
    Some(&["Alice"]),    // include_names
    None, None, None,
);
```

## Trimming messages with `trim_messages`

The `trim_messages` function trims a message list to fit within a token budget. It supports two strategies: keep the first messages or keep the last messages.

```rust
use synaptic::core::{trim_messages, TrimStrategy, Message};
```

### Keep last messages (most common)

This is the typical pattern for chat applications where you want to preserve the most recent context:

```rust
let messages = vec![
    Message::system("You are a helpful assistant."),
    Message::human("Question 1"),
    Message::ai("Answer 1"),
    Message::human("Question 2"),
    Message::ai("Answer 2"),
    Message::human("Question 3"),
];

// Simple token counter: estimate ~4 chars per token
let token_counter = |msg: &Message| -> usize {
    msg.content().len() / 4
};

// Keep last messages within 50 tokens, preserve the system message
let trimmed = trim_messages(
    messages,
    50,               // max_tokens
    token_counter,
    TrimStrategy::Last,
    true,             // include_system: preserve the leading system message
);

// Result: system message + as many recent messages as fit in the budget
assert!(trimmed[0].is_system());
```

### Keep first messages

Useful when you want to preserve the beginning of a conversation:

```rust
let trimmed = trim_messages(
    messages,
    50,
    token_counter,
    TrimStrategy::First,
    false,  // include_system not relevant for First strategy
);
```

### The `include_system` parameter

When using `TrimStrategy::Last` with `include_system: true`:

1. If the first message is a system message, it is always preserved.
2. The system message's tokens are subtracted from the budget.
3. The remaining budget is filled with messages from the end of the list.

This ensures your system prompt is never trimmed away, even as the conversation grows.

### Custom token counters

The `token_counter` parameter is a function that takes a `&Message` and returns a `usize` token count. You can use any estimation strategy:

```rust
// Simple character-based estimate
let simple = |msg: &Message| -> usize { msg.content().len() / 4 };

// Word-based estimate
let word_based = |msg: &Message| -> usize {
    msg.content().split_whitespace().count()
};

// Fixed cost per message (useful when all messages are similar size)
let fixed = |_msg: &Message| -> usize { 10 };
```
