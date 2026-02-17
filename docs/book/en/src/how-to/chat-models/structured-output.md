# Structured Output

This guide shows how to get typed Rust structs from LLM responses using `StructuredOutputChatModel<T>`.

## Overview

`StructuredOutputChatModel<T>` wraps any `ChatModel` and instructs it to respond with valid JSON matching a schema you describe. It injects a system prompt with the schema instructions and provides a `parse_response()` method to deserialize the JSON into your Rust type.

## Basic usage

Define your output type as a struct that implements `Deserialize`, then wrap your model:

```rust
use std::sync::Arc;
use serde::Deserialize;
use synaptic_core::{ChatModel, ChatRequest, Message};
use synaptic_models::StructuredOutputChatModel;

#[derive(Debug, Deserialize)]
struct MovieReview {
    title: String,
    rating: f32,
    summary: String,
}

async fn get_review(base_model: Arc<dyn ChatModel>) -> Result<(), Box<dyn std::error::Error>> {
    let structured = StructuredOutputChatModel::<MovieReview>::new(
        base_model,
        r#"{"title": "string", "rating": "number (1-10)", "summary": "string"}"#,
    );

    let request = ChatRequest::new(vec![
        Message::human("Review the movie 'Interstellar'"),
    ]);

    // Use generate() to get both the parsed struct and the raw response
    let (review, _raw_response) = structured.generate(request).await?;

    println!("Title: {}", review.title);
    println!("Rating: {}/10", review.rating);
    println!("Summary: {}", review.summary);

    Ok(())
}
```

## How it works

When you call `chat()` or `generate()` on a `StructuredOutputChatModel`:

1. A system message is prepended to the request instructing the model to respond with valid JSON matching the schema description.
2. The request is forwarded to the inner model.
3. With `generate()`, the response text is parsed as JSON into your target type `T`.

The schema description is a free-form string. It does not need to be valid JSON Schema -- it just needs to clearly communicate the expected shape to the LLM:

```rust
// Simple field descriptions
let schema = r#"{"name": "string", "age": "integer", "hobbies": ["string"]}"#;

// More detailed descriptions
let schema = r#"{
    "sentiment": "one of: positive, negative, neutral",
    "confidence": "float between 0.0 and 1.0",
    "key_phrases": "array of strings"
}"#;
```

## Parsing responses manually

If you want to use the model as a normal `ChatModel` and parse later, you can call `chat()` followed by `parse_response()`:

```rust
let structured = StructuredOutputChatModel::<MovieReview>::new(base_model, schema);

let response = structured.chat(request).await?;
let parsed: MovieReview = structured.parse_response(&response)?;
```

## Handling markdown code blocks

The parser automatically handles responses wrapped in markdown code blocks. All of these formats are supported:

```text
{"title": "Interstellar", "rating": 9.0, "summary": "..."}
```

````text
```json
{"title": "Interstellar", "rating": 9.0, "summary": "..."}
```
````

````text
```
{"title": "Interstellar", "rating": 9.0, "summary": "..."}
```
````

## Complex output types

You can use nested structs, enums, and collections:

```rust
#[derive(Debug, Deserialize)]
struct AnalysisResult {
    entities: Vec<Entity>,
    sentiment: Sentiment,
    language: String,
}

#[derive(Debug, Deserialize)]
struct Entity {
    name: String,
    entity_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

let structured = StructuredOutputChatModel::<AnalysisResult>::new(
    base_model,
    r#"{
        "entities": [{"name": "string", "entity_type": "person|org|location"}],
        "sentiment": "positive|negative|neutral",
        "language": "ISO 639-1 code"
    }"#,
);
```

## Combining with other wrappers

Since `StructuredOutputChatModel<T>` implements `ChatModel`, it composes with other wrappers:

```rust
use synaptic_models::{RetryChatModel, RetryPolicy};

let base: Arc<dyn ChatModel> = Arc::new(base_model);
let structured = Arc::new(StructuredOutputChatModel::<MovieReview>::new(
    base,
    r#"{"title": "string", "rating": "number", "summary": "string"}"#,
));

// Add retry logic on top
let reliable = RetryChatModel::new(structured, RetryPolicy::default());
```
