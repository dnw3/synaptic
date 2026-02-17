# Error Handling

Synapse uses a single error enum, `SynapseError`, across the entire framework. Every async function returns `Result<T, SynapseError>`, and errors propagate naturally with the `?` operator. This page explains the error model, the available variants, and the patterns for handling and recovering from errors.

## SynapseError

```rust
#[derive(Debug, Error)]
pub enum SynapseError {
    #[error("prompt error: {0}")]           Prompt(String),
    #[error("model error: {0}")]            Model(String),
    #[error("tool error: {0}")]             Tool(String),
    #[error("tool not found: {0}")]         ToolNotFound(String),
    #[error("memory error: {0}")]           Memory(String),
    #[error("rate limit: {0}")]             RateLimit(String),
    #[error("timeout: {0}")]                Timeout(String),
    #[error("validation error: {0}")]       Validation(String),
    #[error("parsing error: {0}")]          Parsing(String),
    #[error("callback error: {0}")]         Callback(String),
    #[error("max steps exceeded: {max_steps}")]  MaxStepsExceeded { max_steps: usize },
    #[error("embedding error: {0}")]        Embedding(String),
    #[error("vector store error: {0}")]     VectorStore(String),
    #[error("retriever error: {0}")]        Retriever(String),
    #[error("loader error: {0}")]           Loader(String),
    #[error("splitter error: {0}")]         Splitter(String),
    #[error("graph error: {0}")]            Graph(String),
    #[error("cache error: {0}")]            Cache(String),
    #[error("config error: {0}")]           Config(String),
}
```

Nineteen variants, one for each subsystem. The design is intentional:

- **Single type everywhere**: You never need to convert between error types. Any function in any crate can return `SynapseError`, and the caller can propagate it with `?` without conversion.
- **String payloads**: Most variants carry a `String` message. This keeps the error type simple and avoids nested error hierarchies. The message provides context about what went wrong.
- **`thiserror` derivation**: `SynapseError` implements `std::error::Error` and `Display` automatically via the `#[error(...)]` attributes.

## Variant Reference

### Infrastructure Errors

| Variant | When It Occurs |
|---------|----------------|
| `Model(String)` | LLM provider returns an error, network failure, invalid response format |
| `RateLimit(String)` | Provider rate limit exceeded, token bucket exhausted |
| `Timeout(String)` | Request timed out |
| `Config(String)` | Invalid configuration (missing API key, bad parameters) |

### Input/Output Errors

| Variant | When It Occurs |
|---------|----------------|
| `Prompt(String)` | Template variable missing, invalid template syntax |
| `Validation(String)` | Input fails validation (e.g., empty message list, invalid schema) |
| `Parsing(String)` | Output parser cannot extract structured data from LLM response |

### Tool Errors

| Variant | When It Occurs |
|---------|----------------|
| `Tool(String)` | Tool execution failed (network error, computation error, etc.) |
| `ToolNotFound(String)` | Requested tool name is not in the registry |

### Subsystem Errors

| Variant | When It Occurs |
|---------|----------------|
| `Memory(String)` | Memory store read/write failure |
| `Callback(String)` | Callback handler raised an error |
| `Embedding(String)` | Embedding API failure |
| `VectorStore(String)` | Vector store read/write failure |
| `Retriever(String)` | Retrieval operation failed |
| `Loader(String)` | Document loading failed (file not found, parse error) |
| `Splitter(String)` | Text splitting failed |
| `Cache(String)` | Cache read/write failure |

### Execution Control Errors

| Variant | When It Occurs |
|---------|----------------|
| `Graph(String)` | Graph execution error, including interrupts for human-in-the-loop |
| `MaxStepsExceeded { max_steps }` | Agent loop exceeded the maximum iteration count |

## Error Propagation

Because every async function in Synapse returns `Result<T, SynapseError>`, errors propagate naturally:

```rust
async fn process_query(model: &dyn ChatModel, query: &str) -> Result<String, SynapseError> {
    let messages = vec![Message::human(query)];
    let request = ChatRequest::new(messages);
    let response = model.chat(request).await?;  // Model error propagates
    Ok(response.message.content().to_string())
}
```

There is no need for `.map_err()` conversions in application code. A `Model` error from a provider adapter, a `Tool` error from execution, or a `Graph` error from the state machine all flow through the same `Result` type.

## Retry and Fallback Patterns

Not all errors are fatal. Synapse provides several mechanisms for resilience:

### RetryChatModel

Wraps a `ChatModel` and retries on transient failures:

```rust
use synaptic::models::RetryChatModel;

let robust_model = RetryChatModel::new(model, max_retries, delay);
```

On failure, it waits and retries up to `max_retries` times. This handles transient network errors and rate limits without application code needing to implement retry logic.

### RateLimitedChatModel and TokenBucketChatModel

Proactively prevent rate limit errors by throttling requests:

- `RateLimitedChatModel` limits requests per time window.
- `TokenBucketChatModel` uses a token bucket algorithm for smooth rate limiting.

By throttling before hitting the provider's limit, these wrappers convert potential `RateLimit` errors into controlled delays.

### RunnableWithFallbacks

Tries alternative runnables when the primary one fails:

```rust
use synaptic::runnables::RunnableWithFallbacks;

let chain = RunnableWithFallbacks::new(
    primary.boxed(),
    vec![fallback_1.boxed(), fallback_2.boxed()],
);
```

If `primary` fails, `fallback_1` is tried with the same input. If that also fails, `fallback_2` is tried. Only if all options fail does the error propagate.

### RunnableRetry

Retries a runnable with configurable backoff:

```rust
use synaptic::runnables::{RunnableRetry, RetryPolicy};

let retry = RunnableRetry::new(
    flaky_step.boxed(),
    RetryPolicy {
        max_retries: 3,
        delay: Duration::from_millis(200),
        backoff_factor: 2.0,
    },
);
```

The delay doubles after each attempt (200ms, 400ms, 800ms). This is useful for any step in an LCEL chain, not just model calls.

### HandleErrorTool

Wraps a tool so that errors are returned as string results instead of propagating:

```rust
use synaptic::tools::HandleErrorTool;

let safe_tool = HandleErrorTool::new(risky_tool);
```

When the inner tool fails, the error message becomes the tool's output. The LLM sees the error and can decide to retry with different arguments or take a different approach. This prevents a single tool failure from crashing the entire agent loop.

## Graph Interrupts as Errors

Human-in-the-loop interrupts in the graph system are implemented as `SynapseError::Graph` errors with descriptive messages:

```rust
Err(SynapseError::Graph("interrupted before node 'tools'"))
Err(SynapseError::Graph("interrupted after node 'agent'"))
```

This is a deliberate design choice. An interrupt is not a failure -- it is a control flow signal. The graph has checkpointed its state and is waiting for human input. Application code can match on the error message to distinguish interrupts from true errors:

```rust
match graph.invoke(state).await {
    Ok(final_state) => handle_result(final_state),
    Err(SynapseError::Graph(msg)) if msg.starts_with("interrupted") => {
        // Human-in-the-loop: inspect state, get approval, resume
    }
    Err(e) => return Err(e),
}
```

## Matching on Error Variants

Since `SynapseError` is an enum, you can match on specific variants to implement targeted error handling:

```rust
match result {
    Ok(value) => use_value(value),
    Err(SynapseError::RateLimit(_)) => {
        // Wait and retry
    }
    Err(SynapseError::ToolNotFound(name)) => {
        // Log the missing tool and continue without it
    }
    Err(SynapseError::Parsing(msg)) => {
        // LLM output was malformed; ask the model to try again
    }
    Err(e) => {
        // All other errors: propagate
        return Err(e);
    }
}
```

This pattern is especially useful in agent loops where some errors are recoverable (the model can try again) and others are not (network is down, API key is invalid).
