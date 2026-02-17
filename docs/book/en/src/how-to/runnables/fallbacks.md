# Fallbacks

This guide shows how to use `RunnableWithFallbacks` to provide alternative runnables that are tried when the primary one fails.

## Overview

`RunnableWithFallbacks` wraps a primary runnable and a list of fallback runnables. When invoked, it tries the primary first. If the primary returns an error, it tries each fallback in order until one succeeds. If all fail, it returns the error from the last fallback attempted.

This is particularly useful when working with LLM providers that may experience transient outages, or when you want to try a cheaper model first and fall back to a more capable one.

## Basic usage

```rust
use synaptic_runnables::{Runnable, RunnableWithFallbacks, RunnableLambda};
use synaptic_core::{RunnableConfig, SynapticError};

// A runnable that always fails
let unreliable = RunnableLambda::new(|_x: String| async move {
    Err::<String, _>(SynapticError::Provider("service unavailable".into()))
});

// A reliable fallback
let fallback = RunnableLambda::new(|x: String| async move {
    Ok(format!("Fallback handled: {x}"))
});

let with_fallbacks = RunnableWithFallbacks::new(
    unreliable.boxed(),
    vec![fallback.boxed()],
);

let config = RunnableConfig::default();
let result = with_fallbacks.invoke("hello".to_string(), &config).await?;
assert_eq!(result, "Fallback handled: hello");
```

## Multiple fallbacks

You can provide multiple fallbacks. They are tried in order:

```rust
let primary = failing_model.boxed();
let fallback_1 = cheaper_model.boxed();
let fallback_2 = local_model.boxed();

let resilient = RunnableWithFallbacks::new(
    primary,
    vec![fallback_1, fallback_2],
);

// Tries: primary -> fallback_1 -> fallback_2
let result = resilient.invoke(input, &config).await?;
```

If the primary succeeds, no fallbacks are attempted. If the primary fails but `fallback_1` succeeds, `fallback_2` is never tried.

## Input cloning requirement

The input type must implement `Clone`, because `RunnableWithFallbacks` needs to pass a copy of the input to each fallback attempt. This is enforced by the type signature:

```rust
pub struct RunnableWithFallbacks<I: Send + Clone + 'static, O: Send + 'static> {
    primary: BoxRunnable<I, O>,
    fallbacks: Vec<BoxRunnable<I, O>>,
}
```

`String`, `Vec<Message>`, `serde_json::Value`, and most standard types implement `Clone`.

## Streaming with fallbacks

`RunnableWithFallbacks` also supports `stream()`. When streaming, it buffers the primary stream's output. If the primary stream yields an error, it discards the buffered items and tries the next fallback's stream. This means there is no partial output from a failed provider -- you get the complete output from whichever provider succeeds.

```rust
use futures::StreamExt;

let resilient = RunnableWithFallbacks::new(primary.boxed(), vec![fallback.boxed()]);

let mut stream = resilient.stream("input".to_string(), &config);
while let Some(result) = stream.next().await {
    let value = result?;
    println!("Got: {value}");
}
```

## In a chain

`RunnableWithFallbacks` implements `Runnable<I, O>`, so it composes with the pipe operator:

```rust
let resilient_model = RunnableWithFallbacks::new(
    primary_model.boxed(),
    vec![fallback_model.boxed()],
);

let chain = preprocess.boxed() | resilient_model.boxed() | postprocess.boxed();
```

## When to use fallbacks vs. retry

- Use **`RunnableWithFallbacks`** when you have genuinely different alternatives (e.g., different LLM providers or different strategies).
- Use **`RunnableRetry`** when you want to retry the same runnable with exponential backoff (e.g., transient network errors).

You can combine both -- wrap a retrying runnable as the primary, with a different provider as a fallback:

```rust
use synaptic_runnables::{RunnableRetry, RetryPolicy, RunnableWithFallbacks};

let retrying_primary = RunnableRetry::new(primary.boxed(), RetryPolicy::default());
let resilient = RunnableWithFallbacks::new(
    retrying_primary.boxed(),
    vec![fallback.boxed()],
);
```
