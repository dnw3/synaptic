# Output Parsers

Output parsers transform raw LLM output into structured data. Every parser in Synaptic implements the `Runnable` trait, so they compose naturally with prompt templates, chat models, and other runnables using the LCEL pipe operator (`|`).

## Available Parsers

| Parser | Input | Output | Description |
|--------|-------|--------|-------------|
| `StrOutputParser` | `Message` | `String` | Extracts the text content from a message |
| `JsonOutputParser` | `String` | `serde_json::Value` | Parses a string as JSON |
| `StructuredOutputParser<T>` | `String` | `T` | Deserializes JSON into a typed struct |
| `ListOutputParser` | `String` | `Vec<String>` | Splits by a configurable separator |
| `EnumOutputParser` | `String` | `String` | Validates against a list of allowed values |
| `BooleanOutputParser` | `String` | `bool` | Parses yes/no/true/false strings |
| `MarkdownListOutputParser` | `String` | `Vec<String>` | Parses markdown bullet lists |
| `NumberedListOutputParser` | `String` | `Vec<String>` | Parses numbered lists |
| `XmlOutputParser` | `String` | `XmlElement` | Parses XML into a tree structure |

All parsers also implement the `FormatInstructions` trait, which provides a `get_format_instructions()` method. You can include these instructions in your prompt to guide the LLM toward producing output in the expected format.

## Quick Example

```rust
use synaptic_parsers::StrOutputParser;
use synaptic_runnables::Runnable;
use synaptic_core::{Message, RunnableConfig};

let parser = StrOutputParser;
let config = RunnableConfig::default();
let result = parser.invoke(Message::ai("Hello world"), &config).await?;
assert_eq!(result, "Hello world");
```

## Sub-Pages

- [Basic Parsers](basic-parsers.md) -- StrOutputParser, JsonOutputParser, ListOutputParser
- [Structured Parser](structured-parser.md) -- deserialize JSON into typed Rust structs
- [Enum Parser](enum-parser.md) -- validate output against a fixed set of values
