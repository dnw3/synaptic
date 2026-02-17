# Basic Parsers

Synapse provides several simple output parsers for common transformations. Each implements `Runnable`, so it can be used standalone or composed in a pipeline.

## StrOutputParser

Extracts the text content from a `Message`. This is the most commonly used parser -- it sits at the end of most chains to convert the model's response into a plain `String`.

**Signature:** `Runnable<Message, String>`

```rust
use synapse_parsers::StrOutputParser;
use synapse_runnables::Runnable;
use synapse_core::{Message, RunnableConfig};

let parser = StrOutputParser;
let config = RunnableConfig::default();

let result = parser.invoke(Message::ai("Hello world"), &config).await?;
assert_eq!(result, "Hello world");
```

`StrOutputParser` works with any `Message` variant -- system, human, AI, or tool messages all have content that can be extracted.

## JsonOutputParser

Parses a JSON string into a `serde_json::Value`. Useful when you need to work with arbitrary JSON structures without defining a specific Rust type.

**Signature:** `Runnable<String, serde_json::Value>`

```rust
use synapse_parsers::JsonOutputParser;
use synapse_runnables::Runnable;
use synapse_core::RunnableConfig;

let parser = JsonOutputParser;
let config = RunnableConfig::default();

let result = parser.invoke(
    r#"{"name": "Synapse", "version": 1}"#.to_string(),
    &config,
).await?;

assert_eq!(result["name"], "Synapse");
assert_eq!(result["version"], 1);
```

If the input is not valid JSON, the parser returns `Err(SynapseError::Parsing(...))`.

## ListOutputParser

Splits a string into a `Vec<String>` using a configurable separator. Useful when you ask the LLM to return a comma-separated or newline-separated list.

**Signature:** `Runnable<String, Vec<String>>`

```rust
use synapse_parsers::{ListOutputParser, ListSeparator};
use synapse_runnables::Runnable;
use synapse_core::RunnableConfig;

let config = RunnableConfig::default();

// Split on commas
let parser = ListOutputParser::comma();
let result = parser.invoke("apple, banana, cherry".to_string(), &config).await?;
assert_eq!(result, vec!["apple", "banana", "cherry"]);

// Split on newlines (default)
let parser = ListOutputParser::newline();
let result = parser.invoke("first\nsecond\nthird".to_string(), &config).await?;
assert_eq!(result, vec!["first", "second", "third"]);

// Custom separator
let parser = ListOutputParser::new(ListSeparator::Custom("|".to_string()));
let result = parser.invoke("a | b | c".to_string(), &config).await?;
assert_eq!(result, vec!["a", "b", "c"]);
```

Each item is trimmed of leading and trailing whitespace. Empty items after trimming are filtered out.

## Format Instructions

All parsers implement the `FormatInstructions` trait. You can include the instructions in your prompt to guide the model:

```rust
use synapse_parsers::{JsonOutputParser, ListOutputParser, FormatInstructions};

let json_parser = JsonOutputParser;
println!("{}", json_parser.get_format_instructions());
// "Your response should be a valid JSON object."

let list_parser = ListOutputParser::comma();
println!("{}", list_parser.get_format_instructions());
// "Your response should be a list of items separated by commas."
```

## Pipeline Example

A typical chain pipes a prompt template through a model and into a parser:

```rust
use std::collections::HashMap;
use serde_json::json;
use synapse_core::{ChatResponse, Message, RunnableConfig};
use synapse_models::ScriptedChatModel;
use synapse_prompts::{ChatPromptTemplate, MessageTemplate};
use synapse_parsers::StrOutputParser;
use synapse_runnables::Runnable;

let model = ScriptedChatModel::new(vec![
    ChatResponse {
        message: Message::ai("The answer is 42."),
        usage: None,
    },
]);

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system("You are a helpful assistant."),
    MessageTemplate::human("{{ question }}"),
]);

// template -> model -> parser
let chain = template.boxed() | model.boxed() | StrOutputParser.boxed();

let config = RunnableConfig::default();
let values: HashMap<String, serde_json::Value> = HashMap::from([
    ("question".to_string(), json!("What is the meaning of life?")),
]);

let result: String = chain.invoke(values, &config).await?;
assert_eq!(result, "The answer is 42.");
```
