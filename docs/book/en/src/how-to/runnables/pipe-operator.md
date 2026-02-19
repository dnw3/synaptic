# Pipe Operator

This guide shows how to chain runnables together using the `|` pipe operator to build sequential processing pipelines.

## Overview

The `|` operator on `BoxRunnable` creates a `RunnableSequence` that feeds the output of the first runnable into the input of the second. This is the primary way to build LCEL chains in Synaptic.

The pipe operator is implemented via Rust's `BitOr` trait on `BoxRunnable`. Both sides must be boxed first with `.boxed()`, because the operator needs type-erased wrappers to connect runnables with different concrete types.

## Basic chaining

```rust
use synaptic::runnables::{Runnable, RunnableLambda, BoxRunnable};
use synaptic::core::RunnableConfig;

let step1 = RunnableLambda::new(|x: String| async move {
    Ok(format!("Step 1: {x}"))
});

let step2 = RunnableLambda::new(|x: String| async move {
    Ok(format!("{x} -> Step 2"))
});

// Pipe operator creates a RunnableSequence
let chain = step1.boxed() | step2.boxed();

let config = RunnableConfig::default();
let result = chain.invoke("input".to_string(), &config).await?;
assert_eq!(result, "Step 1: input -> Step 2");
```

The types must be compatible: the output type of `step1` must match the input type of `step2`. In this example both work with `String`, so the types line up. The compiler will reject chains where the types do not match.

## Multi-step chains

You can chain more than two steps by continuing to pipe. The result is still a single `BoxRunnable`:

```rust
let step3 = RunnableLambda::new(|x: String| async move {
    Ok(format!("{x} -> Step 3"))
});

let chain = step1.boxed() | step2.boxed() | step3.boxed();

let result = chain.invoke("start".to_string(), &config).await?;
assert_eq!(result, "Step 1: start -> Step 2 -> Step 3");
```

Each `|` wraps the left side into a new `RunnableSequence`, so `a | b | c` produces a `RunnableSequence(RunnableSequence(a, b), c)`. This nesting is transparent -- you interact with the result as a single `BoxRunnable<I, O>`.

## Type conversions across steps

Steps can change the type flowing through the chain, as long as each step's output matches the next step's input:

```rust
use synaptic::runnables::{Runnable, RunnableLambda};
use synaptic::core::RunnableConfig;

// String -> usize -> String
let count_chars = RunnableLambda::new(|s: String| async move {
    Ok(s.len())
});

let format_count = RunnableLambda::new(|n: usize| async move {
    Ok(format!("Length: {n}"))
});

let chain = count_chars.boxed() | format_count.boxed();

let config = RunnableConfig::default();
let result = chain.invoke("hello".to_string(), &config).await?;
assert_eq!(result, "Length: 5");
```

## Why `boxed()` is required

Rust's type system needs to know the exact types at compile time. Without `boxed()`, each `RunnableLambda` has a unique closure type that cannot appear on both sides of `|`. Calling `.boxed()` erases the concrete type into `BoxRunnable<I, O>`, which is a trait object that can compose with any other `BoxRunnable` as long as the input/output types align.

`BoxRunnable::new(runnable)` is equivalent to `runnable.boxed()` -- use whichever reads better in context.

## Using `RunnablePassthrough`

`RunnablePassthrough` is a no-op runnable that passes its input through unchanged. It is useful when you need an identity step in a chain -- for example, as one branch in a `RunnableParallel`:

```rust
use synaptic::runnables::{Runnable, RunnablePassthrough};

let passthrough = RunnablePassthrough;
let result = passthrough.invoke("unchanged".to_string(), &config).await?;
assert_eq!(result, "unchanged");
```

## Error propagation

If any step in the chain returns an `Err`, the chain short-circuits immediately and returns that error. Subsequent steps are not executed:

```rust
use synaptic::core::SynapticError;

let failing = RunnableLambda::new(|_x: String| async move {
    Err::<String, _>(SynapticError::Validation("something went wrong".into()))
});

let after = RunnableLambda::new(|x: String| async move {
    Ok(format!("This won't run: {x}"))
});

let chain = failing.boxed() | after.boxed();
let result = chain.invoke("test".to_string(), &config).await;
assert!(result.is_err());
```
