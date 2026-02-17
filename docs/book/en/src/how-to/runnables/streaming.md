# Streaming through Chains

This guide shows how to use `stream()` to consume incremental output from an LCEL chain.

## Overview

Every `Runnable` provides a `stream()` method that returns a `RunnableOutputStream` -- a pinned, boxed `Stream` of `Result<O, SynapticError>` items. This allows downstream consumers to process results as they become available, rather than waiting for the entire chain to finish.

The default `stream()` implementation wraps `invoke()` as a single-item stream. Runnables that support true incremental output (such as LLM model adapters or `RunnableGenerator`) override `stream()` to yield items one at a time.

## Streaming a single runnable

```rust
use futures::StreamExt;
use synaptic_runnables::{Runnable, RunnableLambda};
use synaptic_core::RunnableConfig;

let upper = RunnableLambda::new(|x: String| async move {
    Ok(x.to_uppercase())
});

let config = RunnableConfig::default();
let mut stream = upper.stream("hello".to_string(), &config);

while let Some(result) = stream.next().await {
    let value = result?;
    println!("Got: {value}");
}
// Prints: Got: HELLO
```

Because `RunnableLambda` uses the default `stream()` implementation, this yields exactly one item -- the full result of `invoke()`.

## Streaming through a chain

When you stream through a `RunnableSequence` (created by the `|` operator), the behavior is:

1. The first step runs fully via `invoke()` and produces its complete output.
2. That output is fed into the second step's `stream()`, which yields items incrementally.

This means **only the final component in a chain truly streams**. Intermediate steps buffer their output. This matches the LangChain behavior.

```rust
use futures::StreamExt;
use synaptic_runnables::{Runnable, RunnableLambda};
use synaptic_core::RunnableConfig;

let step1 = RunnableLambda::new(|x: String| async move {
    Ok(format!("processed: {x}"))
});

let step2 = RunnableLambda::new(|x: String| async move {
    Ok(x.to_uppercase())
});

let chain = step1.boxed() | step2.boxed();

let config = RunnableConfig::default();
let mut stream = chain.stream("input".to_string(), &config);

while let Some(result) = stream.next().await {
    let value = result?;
    println!("Got: {value}");
}
// Prints: Got: PROCESSED: INPUT
```

## Streaming with `BoxRunnable`

`BoxRunnable` preserves the streaming behavior of the inner runnable. Call `.stream()` directly on it:

```rust
let boxed_chain = step1.boxed() | step2.boxed();
let mut stream = boxed_chain.stream("input".to_string(), &config);

while let Some(result) = stream.next().await {
    let value = result?;
    println!("{value}");
}
```

## True streaming with `RunnableGenerator`

`RunnableGenerator` wraps a generator function that returns a `Stream`, enabling true incremental output:

```rust
use futures::StreamExt;
use synaptic_runnables::{Runnable, RunnableGenerator};
use synaptic_core::RunnableConfig;

let gen = RunnableGenerator::new(|input: String| {
    async_stream::stream! {
        for word in input.split_whitespace() {
            yield Ok(word.to_uppercase());
        }
    }
});

let config = RunnableConfig::default();
let mut stream = gen.stream("hello world foo".to_string(), &config);

while let Some(result) = stream.next().await {
    let items = result?;
    println!("Chunk: {:?}", items);
}
// Prints each word as a separate chunk:
// Chunk: ["HELLO"]
// Chunk: ["WORLD"]
// Chunk: ["FOO"]
```

When you call `invoke()` on a `RunnableGenerator`, it collects all streamed items into a `Vec<O>`.

## Collecting a stream into a single result

If you need the full result rather than incremental output, use `invoke()` instead of `stream()`. Alternatively, collect the stream manually:

```rust
use futures::StreamExt;

let mut stream = chain.stream("input".to_string(), &config);
let mut items = Vec::new();

while let Some(result) = stream.next().await {
    items.push(result?);
}

// items now contains all yielded values
```

## Error handling in streams

If any step in a chain fails during streaming, the stream yields an `Err` item. Consumers should check each item:

```rust
while let Some(result) = stream.next().await {
    match result {
        Ok(value) => println!("Got: {value}"),
        Err(e) => eprintln!("Error: {e}"),
    }
}
```

When the first step of a `RunnableSequence` fails (during its `invoke()`), the stream immediately yields that error as the only item.
