# Chat Prompt Template

`ChatPromptTemplate` produces a `Vec<Message>` from a sequence of `MessageTemplate` entries. Each entry renders one or more messages with `{{ variable }}` interpolation. The template implements the `Runnable` trait, so it integrates directly into LCEL pipelines.

## Creating a Template

Use `ChatPromptTemplate::from_messages()` (or `new()`) with a vector of `MessageTemplate` variants:

```rust
use synapse_prompts::{ChatPromptTemplate, MessageTemplate};

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are a {{ role }} assistant."),
    MessageTemplate::human("{{ question }}"),
]);
```

## Rendering with `format()`

Call `format()` with a `HashMap<String, serde_json::Value>` to produce messages:

```rust
use std::collections::HashMap;
use serde_json::json;
use synapse_prompts::{ChatPromptTemplate, MessageTemplate};

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are a {{ role }} assistant."),
    MessageTemplate::human("{{ question }}"),
]);

let values: HashMap<String, serde_json::Value> = HashMap::from([
    ("role".to_string(), json!("helpful")),
    ("question".to_string(), json!("What is Rust?")),
]);

let messages = template.format(&values).unwrap();
// messages[0] => Message::system("You are a helpful assistant.")
// messages[1] => Message::human("What is Rust?")
```

## Using as a Runnable

Because `ChatPromptTemplate` implements `Runnable<HashMap<String, Value>, Vec<Message>>`, you can call `invoke()` or compose it with the pipe operator:

```rust
use std::collections::HashMap;
use serde_json::json;
use synapse_core::RunnableConfig;
use synapse_prompts::{ChatPromptTemplate, MessageTemplate};
use synapse_runnables::Runnable;

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are a {{ role }} assistant."),
    MessageTemplate::human("{{ question }}"),
]);

let config = RunnableConfig::default();
let values: HashMap<String, serde_json::Value> = HashMap::from([
    ("role".to_string(), json!("helpful")),
    ("question".to_string(), json!("What is Rust?")),
]);

let messages = template.invoke(values, &config).await?;
// messages = [Message::system("You are a helpful assistant."), Message::human("What is Rust?")]
```

## MessageTemplate Variants

`MessageTemplate` is an enum with four variants:

| Variant | Description |
|---------|-------------|
| `MessageTemplate::system(text)` | Renders a system message from a template string |
| `MessageTemplate::human(text)` | Renders a human message from a template string |
| `MessageTemplate::ai(text)` | Renders an AI message from a template string |
| `MessageTemplate::Placeholder(key)` | Injects a list of messages from the input map |

### Placeholder Example

`Placeholder` injects messages stored under a key in the input map. The value must be a JSON array of serialized `Message` objects. This is useful for injecting conversation history:

```rust
use std::collections::HashMap;
use serde_json::json;
use synapse_prompts::{ChatPromptTemplate, MessageTemplate};

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are helpful."),
    MessageTemplate::Placeholder("history".to_string()),
    MessageTemplate::human("{{ input }}"),
]);

let history = json!([
    {"role": "human", "content": "Hi"},
    {"role": "assistant", "content": "Hello!"}
]);

let values: HashMap<String, serde_json::Value> = HashMap::from([
    ("history".to_string(), history),
    ("input".to_string(), json!("How are you?")),
]);

let messages = template.format(&values).unwrap();
// messages[0] => System("You are helpful.")
// messages[1] => Human("Hi")         -- from placeholder
// messages[2] => AI("Hello!")         -- from placeholder
// messages[3] => Human("How are you?")
```

## Composing in a Pipeline

A common pattern is to pipe a prompt template into a chat model and then into an output parser:

```rust
use std::collections::HashMap;
use serde_json::json;
use synapse_core::{ChatModel, ChatResponse, Message, RunnableConfig};
use synapse_models::ScriptedChatModel;
use synapse_prompts::{ChatPromptTemplate, MessageTemplate};
use synapse_parsers::StrOutputParser;
use synapse_runnables::Runnable;

let model = ScriptedChatModel::new(vec![
    ChatResponse {
        message: Message::ai("Rust is a systems programming language."),
        usage: None,
    },
]);

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are a {{ role }} assistant."),
    MessageTemplate::human("{{ question }}"),
]);

let chain = template.boxed() | model.boxed() | StrOutputParser.boxed();

let values: HashMap<String, serde_json::Value> = HashMap::from([
    ("role".to_string(), json!("helpful")),
    ("question".to_string(), json!("What is Rust?")),
]);

let config = RunnableConfig::default();
let result: String = chain.invoke(values, &config).await.unwrap();
// result = "Rust is a systems programming language."
```
