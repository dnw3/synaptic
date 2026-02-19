# Retry & Rate Limiting

This guide shows how to add automatic retry logic and rate limiting to any `ChatModel`.

## Retry with `RetryChatModel`

`RetryChatModel` wraps a model and automatically retries on transient failures (rate limit errors and timeouts). It uses exponential backoff between attempts.

```rust
use std::sync::Arc;
use synaptic::core::ChatModel;
use synaptic::models::{RetryChatModel, RetryPolicy};

let base_model: Arc<dyn ChatModel> = Arc::new(model);

// Use default policy: 3 attempts, 500ms base delay
let retry_model = RetryChatModel::new(base_model, RetryPolicy::default());
```

### Custom retry policy

Configure the maximum number of attempts and the base delay for exponential backoff:

```rust
use std::time::Duration;
use synaptic::models::RetryPolicy;

let policy = RetryPolicy {
    max_attempts: 5,                         // Try up to 5 times
    base_delay: Duration::from_millis(200),  // Start with 200ms delay
};

let retry_model = RetryChatModel::new(base_model, policy);
```

The delay between retries follows exponential backoff: `base_delay * 2^attempt`. With a 200ms base delay:

| Attempt | Delay before retry |
|---------|-------------------|
| 1st retry | 200ms |
| 2nd retry | 400ms |
| 3rd retry | 800ms |
| 4th retry | 1600ms |

Only retryable errors trigger retries:
- `SynapticError::RateLimit` -- the provider returned a rate limit response.
- `SynapticError::Timeout` -- the request timed out.

All other errors are returned immediately without retrying.

### Streaming with retry

`RetryChatModel` also retries `stream_chat()` calls. If a retryable error occurs during streaming, the entire stream is retried from the beginning.

## Concurrency limiting with `RateLimitedChatModel`

`RateLimitedChatModel` uses a semaphore to limit the number of concurrent requests to the underlying model:

```rust
use std::sync::Arc;
use synaptic::core::ChatModel;
use synaptic::models::RateLimitedChatModel;

let base_model: Arc<dyn ChatModel> = Arc::new(model);

// Allow at most 5 concurrent requests
let limited = RateLimitedChatModel::new(base_model, 5);
```

When the concurrency limit is reached, additional callers wait until a slot becomes available. This is useful for:

- Respecting provider concurrency limits.
- Preventing resource exhaustion in high-throughput applications.
- Controlling costs by limiting parallel API calls.

## Token bucket rate limiting with `TokenBucketChatModel`

`TokenBucketChatModel` uses a token bucket algorithm for smoother rate limiting. The bucket starts full and refills at a steady rate:

```rust
use std::sync::Arc;
use synaptic::core::ChatModel;
use synaptic::models::TokenBucketChatModel;

let base_model: Arc<dyn ChatModel> = Arc::new(model);

// Bucket capacity: 100 tokens, refill rate: 10 tokens/second
let throttled = TokenBucketChatModel::new(base_model, 100.0, 10.0);
```

Each `chat()` or `stream_chat()` call consumes one token from the bucket. When the bucket is empty, callers wait until a token is refilled.

Parameters:
- **capacity** -- the maximum burst size. A capacity of 100 allows 100 rapid-fire requests before throttling kicks in.
- **refill_rate** -- tokens added per second. A rate of 10.0 means the bucket refills at 10 tokens per second.

### Token bucket vs concurrency limiting

| Feature | `RateLimitedChatModel` | `TokenBucketChatModel` |
|---------|----------------------|----------------------|
| Controls | Concurrent requests | Request rate over time |
| Mechanism | Semaphore | Token bucket |
| Burst handling | Blocks when N requests are in-flight | Allows bursts up to capacity |
| Best for | Concurrency limits | Rate limits (requests/second) |

## Stacking wrappers

All wrappers implement `ChatModel`, so they compose naturally. A common pattern is retry on the outside, rate limiting on the inside:

```rust
use std::sync::Arc;
use synaptic::core::ChatModel;
use synaptic::models::{RetryChatModel, RetryPolicy, TokenBucketChatModel};

let base_model: Arc<dyn ChatModel> = Arc::new(model);

// First, apply rate limiting
let throttled: Arc<dyn ChatModel> = Arc::new(
    TokenBucketChatModel::new(base_model, 50.0, 5.0)
);

// Then, add retry on top
let reliable = RetryChatModel::new(throttled, RetryPolicy::default());
```

This ensures that retried requests also go through the rate limiter, preventing retry storms from overwhelming the provider.
