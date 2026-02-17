# Enum Parser

`EnumOutputParser` validates that the LLM's output matches one of a predefined set of allowed values. This is useful for classification tasks where the model should respond with exactly one of several categories.

## Basic Usage

Create the parser with a list of allowed values, then invoke it:

```rust
use synaptic_parsers::EnumOutputParser;
use synaptic_runnables::Runnable;
use synaptic_core::RunnableConfig;

let parser = EnumOutputParser::new(vec![
    "positive".to_string(),
    "negative".to_string(),
    "neutral".to_string(),
]);

let config = RunnableConfig::default();

// Valid value -- returns Ok
let result = parser.invoke("positive".to_string(), &config).await?;
assert_eq!(result, "positive");
```

**Signature:** `Runnable<String, String>`

## Validation

The parser trims whitespace from the input before checking. If the trimmed input does not match any allowed value, it returns `Err(SynapseError::Parsing(...))`:

```rust
use synaptic_parsers::EnumOutputParser;
use synaptic_runnables::Runnable;
use synaptic_core::RunnableConfig;

let parser = EnumOutputParser::new(vec![
    "positive".to_string(),
    "negative".to_string(),
    "neutral".to_string(),
]);

let config = RunnableConfig::default();

// Whitespace is trimmed -- this succeeds
let result = parser.invoke("  neutral  ".to_string(), &config).await?;
assert_eq!(result, "neutral");

// Invalid value -- returns an error
let err = parser.invoke("invalid".to_string(), &config).await.unwrap_err();
assert!(err.to_string().contains("expected one of"));
```

## Format Instructions

`EnumOutputParser` implements `FormatInstructions`. Include the instructions in your prompt so the model knows which values to choose from:

```rust
use synaptic_parsers::{EnumOutputParser, FormatInstructions};

let parser = EnumOutputParser::new(vec![
    "positive".to_string(),
    "negative".to_string(),
    "neutral".to_string(),
]);

let instructions = parser.get_format_instructions();
// "Your response should be one of the following values: positive, negative, neutral"
```

## Pipeline Example

A typical classification pipeline combines a prompt, a model, a content extractor, and the enum parser:

```rust
use std::collections::HashMap;
use serde_json::json;
use synaptic_core::{ChatResponse, Message, RunnableConfig};
use synaptic_models::ScriptedChatModel;
use synaptic_prompts::{ChatPromptTemplate, MessageTemplate};
use synaptic_parsers::{StrOutputParser, EnumOutputParser, FormatInstructions};
use synaptic_runnables::Runnable;

let parser = EnumOutputParser::new(vec![
    "positive".to_string(),
    "negative".to_string(),
    "neutral".to_string(),
]);

let model = ScriptedChatModel::new(vec![
    ChatResponse {
        message: Message::ai("positive"),
        usage: None,
    },
]);

let template = ChatPromptTemplate::from_messages(vec![
    MessageTemplate::system(
        &format!(
            "Classify the sentiment of the text. {}",
            parser.get_format_instructions()
        ),
    ),
    MessageTemplate::human("{{ text }}"),
]);

// template -> model -> extract content -> validate enum
let chain = template.boxed()
    | model.boxed()
    | StrOutputParser.boxed()
    | parser.boxed();

let config = RunnableConfig::default();
let values: HashMap<String, serde_json::Value> = HashMap::from([
    ("text".to_string(), json!("I love this product!")),
]);

let result: String = chain.invoke(values, &config).await?;
assert_eq!(result, "positive");
```
