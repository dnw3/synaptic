# Prompts

Synaptic provides two levels of prompt template:

- **`PromptTemplate`** -- simple string interpolation with `{{ variable }}` syntax. Takes a `HashMap<String, String>` and returns a rendered `String`.
- **`ChatPromptTemplate`** -- produces a `Vec<Message>` from a sequence of `MessageTemplate` entries. Each entry can be a system, human, or AI message template, or a `Placeholder` that injects an existing list of messages.

Both template types implement the `Runnable` trait, so they compose directly with chat models, output parsers, and other runnables using the LCEL pipe operator (`|`).

## Quick Example

```rust
use synaptic::prompts::{PromptTemplate, ChatPromptTemplate, MessageTemplate};

// Simple string template
let pt = PromptTemplate::new("Hello, {{ name }}!");
let mut values = std::collections::HashMap::new();
values.insert("name".to_string(), "world".to_string());
assert_eq!(pt.render(&values).unwrap(), "Hello, world!");

// Chat message template (produces Vec<Message>)
let chat = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are a {{ role }} assistant."),
    MessageTemplate::human("{{ question }}"),
]);
```

## Sub-Pages

- [Chat Prompt Template](chat-prompt-template.md) -- build multi-message prompts with variable interpolation and placeholders
- [Few-Shot Prompting](few-shot.md) -- inject example conversations for few-shot learning
