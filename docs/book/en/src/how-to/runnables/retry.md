# Retry

This guide shows how to use `RunnableRetry` with `RetryPolicy` to automatically retry a runnable on failure with exponential backoff.

## Overview

`RunnableRetry` wraps any runnable with retry logic. When the inner runnable returns an error, `RunnableRetry` waits for a backoff delay and tries again, up to a configurable maximum number of attempts. The backoff follows an exponential schedule: `min(base_delay * 2^attempt, max_delay)`.

## Basic usage

```rust
use std::time::Duration;
use synaptic::runnables::{Runnable, RunnableRetry, RetryPolicy, RunnableLambda};
use synaptic::core::RunnableConfig;

let flaky_step = RunnableLambda::new(|x: String| async move {
    // Imagine this sometimes fails due to network issues
    Ok(x.to_uppercase())
});

let policy = RetryPolicy::default();  // 3 attempts, 100ms base delay, 10s max delay

let with_retry = RunnableRetry::new(flaky_step.boxed(), policy);

let config = RunnableConfig::default();
let result = with_retry.invoke("hello".to_string(), &config).await?;
assert_eq!(result, "HELLO");
```

## Configuring the retry policy

`RetryPolicy` uses a builder pattern for configuration:

```rust
use std::time::Duration;
use synaptic::runnables::RetryPolicy;

let policy = RetryPolicy::default()
    .with_max_attempts(5)               // Up to 5 total attempts (1 initial + 4 retries)
    .with_base_delay(Duration::from_millis(200))   // Start with 200ms delay
    .with_max_delay(Duration::from_secs(30));      // Cap delay at 30 seconds
```

### Default values

| Field | Default |
|-------|---------|
| `max_attempts` | 3 |
| `base_delay` | 100ms |
| `max_delay` | 10 seconds |

### Backoff schedule

The delay for each retry attempt is calculated as:

```
delay = min(base_delay * 2^attempt, max_delay)
```

For the defaults (100ms base, 10s max):

| Attempt | Delay |
|---------|-------|
| 1st retry (attempt 0) | 100ms |
| 2nd retry (attempt 1) | 200ms |
| 3rd retry (attempt 2) | 400ms |
| 4th retry (attempt 3) | 800ms |
| ... | ... |
| Capped at | 10s |

## Filtering retryable errors

By default, all errors trigger a retry. Use `with_retry_on()` to specify a predicate that decides which errors are worth retrying:

```rust
use synaptic::runnables::RetryPolicy;
use synaptic::core::SynapticError;

let policy = RetryPolicy::default()
    .with_max_attempts(4)
    .with_retry_on(|error: &SynapticError| {
        // Only retry provider errors (e.g., rate limits, timeouts)
        matches!(error, SynapticError::Provider(_))
    });
```

When the predicate returns `false` for an error, `RunnableRetry` immediately returns that error without further retries.

## Input cloning requirement

The input type must implement `Clone`, because the input is reused for each retry attempt:

```rust
pub struct RunnableRetry<I: Send + Clone + 'static, O: Send + 'static> { ... }
```

## In a chain

`RunnableRetry` implements `Runnable<I, O>`, so it works with the pipe operator:

```rust
use synaptic::runnables::{Runnable, RunnableRetry, RetryPolicy, RunnableLambda};

let preprocess = RunnableLambda::new(|x: String| async move {
    Ok(x.trim().to_string())
});

let retrying_model = RunnableRetry::new(
    model_step.boxed(),
    RetryPolicy::default().with_max_attempts(3),
);

let chain = preprocess.boxed() | retrying_model.boxed();
```

## Combining retry with fallbacks

For maximum resilience, wrap a retrying runnable with fallbacks. The primary is retried up to its limit; if it still fails, the fallback is tried:

```rust
use synaptic::runnables::{RunnableRetry, RetryPolicy, RunnableWithFallbacks};

let retrying_primary = RunnableRetry::new(
    primary_model.boxed(),
    RetryPolicy::default().with_max_attempts(3),
);

let resilient = RunnableWithFallbacks::new(
    retrying_primary.boxed(),
    vec![fallback_model.boxed()],
);
```

## Full example

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use synaptic::runnables::{Runnable, RunnableRetry, RetryPolicy, RunnableLambda};
use synaptic::core::{RunnableConfig, SynapticError};

// Simulate a flaky service that fails twice then succeeds
let call_count = Arc::new(AtomicUsize::new(0));
let counter = call_count.clone();

let flaky = RunnableLambda::new(move |x: String| {
    let counter = counter.clone();
    async move {
        let n = counter.fetch_add(1, Ordering::SeqCst);
        if n < 2 {
            Err(SynapticError::Provider("temporary failure".into()))
        } else {
            Ok(format!("Success: {x}"))
        }
    }
});

let policy = RetryPolicy::default()
    .with_max_attempts(5)
    .with_base_delay(Duration::from_millis(10));

let retrying = RunnableRetry::new(flaky.boxed(), policy);

let config = RunnableConfig::default();
let result = retrying.invoke("test".to_string(), &config).await?;
assert_eq!(result, "Success: test");
assert_eq!(call_count.load(Ordering::SeqCst), 3);  // 2 failures + 1 success
```
