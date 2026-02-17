# Structured Parser

`StructuredOutputParser<T>` deserializes a JSON string directly into a typed Rust struct. This is the preferred parser when you know the exact shape of the data you expect from the LLM.

## Basic Usage

Define a struct that derives `Deserialize`, then create a parser for it:

```rust
use synaptic_parsers::StructuredOutputParser;
use synaptic_runnables::Runnable;
use synaptic_core::RunnableConfig;
use serde::Deserialize;

#[derive(Deserialize)]
struct Person {
    name: String,
    age: u32,
}

let parser = StructuredOutputParser::<Person>::new();
let config = RunnableConfig::default();

let result = parser.invoke(
    r#"{"name": "Alice", "age": 30}"#.to_string(),
    &config,
).await?;

assert_eq!(result.name, "Alice");
assert_eq!(result.age, 30);
```

**Signature:** `Runnable<String, T>` where `T: DeserializeOwned + Send + Sync + 'static`

## Error Handling

If the input string is not valid JSON or does not match the struct's schema, the parser returns `Err(SynapseError::Parsing(...))`:

```rust
use synaptic_parsers::StructuredOutputParser;
use synaptic_runnables::Runnable;
use synaptic_core::RunnableConfig;
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    enabled: bool,
    threshold: f64,
}

let parser = StructuredOutputParser::<Config>::new();
let config = RunnableConfig::default();

// Missing required field -- returns an error
let err = parser.invoke(
    r#"{"enabled": true}"#.to_string(),
    &config,
).await.unwrap_err();

assert!(err.to_string().contains("structured parse error"));
```

## Format Instructions

`StructuredOutputParser<T>` implements the `FormatInstructions` trait. Include the instructions in your prompt to guide the model toward producing correctly-shaped JSON:

```rust
use synaptic_parsers::{StructuredOutputParser, FormatInstructions};
use serde::Deserialize;

#[derive(Deserialize)]
struct Answer {
    reasoning: String,
    answer: String,
}

let parser = StructuredOutputParser::<Answer>::new();
let instructions = parser.get_format_instructions();
// "Your response should be a valid JSON object matching the expected schema."
```

## Pipeline Example

In a chain, `StructuredOutputParser` typically follows a `StrOutputParser` step or receives the string content directly. Here is a complete example:

```rust
use synaptic_parsers::StructuredOutputParser;
use synaptic_runnables::{Runnable, RunnableLambda};
use synaptic_core::{Message, RunnableConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Sentiment {
    label: String,
    confidence: f64,
}

// Simulate an LLM that returns JSON in a Message
let extract_content = RunnableLambda::new(|msg: Message| async move {
    Ok(msg.content().to_string())
});

let parser = StructuredOutputParser::<Sentiment>::new();

let chain = extract_content.boxed() | parser.boxed();
let config = RunnableConfig::default();

let input = Message::ai(r#"{"label": "positive", "confidence": 0.95}"#);
let result: Sentiment = chain.invoke(input, &config).await?;

assert_eq!(result.label, "positive");
assert!((result.confidence - 0.95).abs() < f64::EPSILON);
```

## When to Use Structured vs. JSON Parser

- Use `StructuredOutputParser<T>` when you know the exact schema at compile time and want type-safe access to fields.
- Use `JsonOutputParser` when you need to work with arbitrary or dynamic JSON structures where the shape is not known in advance.
