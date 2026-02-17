# Bind

This guide shows how to use `BoxRunnable::bind()` to attach configuration transforms and listeners to a runnable.

## Overview

`bind()` creates a new `BoxRunnable` that applies a transformation to the `RunnableConfig` before each invocation. This is useful for injecting tags, metadata, or other config fields into a runnable without modifying the call site.

Internally, `bind()` wraps the runnable in a `RunnableBind` that calls the transform function on the config, then delegates to the inner runnable with the modified config.

## Basic usage

```rust
use synapse_runnables::{Runnable, RunnableLambda};
use synapse_core::RunnableConfig;

let step = RunnableLambda::new(|x: String| async move {
    Ok(x.to_uppercase())
});

// Bind a config transform that adds a tag
let bound = step.boxed().bind(|mut config| {
    config.tags.push("my-tag".to_string());
    config
});

let config = RunnableConfig::default();
let result = bound.invoke("hello".to_string(), &config).await?;
assert_eq!(result, "HELLO");
// The inner runnable received a config with tags: ["my-tag"]
```

The transform function receives the `RunnableConfig` by value (cloned from the original) and returns the modified config.

## Adding metadata

You can use `bind()` to attach metadata that downstream runnables or callbacks can inspect:

```rust
use serde_json::json;

let bound = step.boxed().bind(|mut config| {
    config.metadata.insert("source".to_string(), json!("user-query"));
    config.metadata.insert("priority".to_string(), json!("high"));
    config
});
```

## Setting a fixed config with `with_config()`

If you want to replace the config entirely rather than modify it, use `with_config()`. This ignores whatever config is passed at invocation time and uses the provided config instead:

```rust
let fixed_config = RunnableConfig {
    tags: vec!["production".to_string()],
    run_name: Some("fixed-pipeline".to_string()),
    ..RunnableConfig::default()
};

let bound = step.boxed().with_config(fixed_config);

// Even if a different config is passed to invoke(), the fixed config is used
let any_config = RunnableConfig::default();
let result = bound.invoke("hello".to_string(), &any_config).await?;
```

## Streaming with bind

`bind()` also applies the config transform during `stream()` calls, not just `invoke()`:

```rust
use futures::StreamExt;

let bound = step.boxed().bind(|mut config| {
    config.tags.push("streaming".to_string());
    config
});

let mut stream = bound.stream("hello".to_string(), &config);
while let Some(result) = stream.next().await {
    let value = result?;
    println!("{value}");
}
```

## Attaching listeners with `with_listeners()`

`with_listeners()` wraps a runnable with before/after callbacks that fire on each invocation. The callbacks receive a reference to the `RunnableConfig`:

```rust
let with_logging = step.boxed().with_listeners(
    |config| {
        println!("Starting run: {:?}", config.run_name);
    },
    |config| {
        println!("Finished run: {:?}", config.run_name);
    },
);

let result = with_logging.invoke("hello".to_string(), &config).await?;
// Prints: Starting run: None
// Prints: Finished run: None
```

Listeners also fire around `stream()` calls -- `on_start` fires before the first item is yielded, and `on_end` fires after the stream completes.

## Composing with bind in a chain

`bind()` returns a `BoxRunnable`, so you can chain it with the pipe operator:

```rust
let tagged_step = step.boxed().bind(|mut config| {
    config.tags.push("step-1".to_string());
    config
});

let chain = tagged_step | next_step.boxed();
let result = chain.invoke("input".to_string(), &config).await?;
```

## RunnableConfig fields reference

The `RunnableConfig` struct has the following fields that you can modify via `bind()`:

| Field | Type | Description |
|-------|------|-------------|
| `tags` | `Vec<String>` | Tags for filtering and categorization |
| `metadata` | `HashMap<String, Value>` | Arbitrary key-value metadata |
| `max_concurrency` | `Option<usize>` | Concurrency limit for batch operations |
| `recursion_limit` | `Option<usize>` | Maximum recursion depth for chains |
| `run_id` | `Option<String>` | Unique identifier for the current run |
| `run_name` | `Option<String>` | Human-readable name for the current run |
