# Circuit Breaker

Middleware implementing the circuit breaker pattern for tool and model calls. When a tool or model fails repeatedly, the circuit breaker stops sending requests to it temporarily, preventing cascading failures and giving the failing service time to recover.

## Constructor

```rust,ignore
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};

let mw = CircuitBreakerMiddleware::new(CircuitBreakerConfig::default());
```

The default configuration opens the circuit after 5 consecutive failures and waits 60 seconds before probing again.

### Custom Configuration

```rust,ignore
use std::time::Duration;
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};

let config = CircuitBreakerConfig {
    failure_threshold: 3,
    recovery_timeout: Duration::from_secs(30),
};
let mw = CircuitBreakerMiddleware::new(config);
```

## State Machine

The circuit breaker follows the standard three-state pattern:

```
         success
  +--------------------+
  |                    |
  v     threshold      |     recovery timeout
Closed ----------> Open -----------------> HalfOpen
  ^                                          |
  |              success                     |
  +------------------------------------------+
                     |
                   failure
                     |
                     v
                   Open
```

- **Closed** -- Normal operation. All requests flow through. Failures are counted.
- **Open** -- The failure threshold has been reached. All requests are immediately rejected with an error. No calls are forwarded to the underlying tool or model.
- **HalfOpen** -- The recovery timeout has elapsed. A single probe request is allowed through. If it succeeds, the circuit transitions back to Closed. If it fails, the circuit reopens.

## CircuitBreakerConfig

| Field               | Type       | Default          | Description                                          |
|---------------------|------------|------------------|------------------------------------------------------|
| `failure_threshold` | `usize`   | `5`              | Consecutive failures before opening the circuit      |
| `recovery_timeout`  | `Duration` | `60s`           | Time to wait in Open state before allowing a probe   |

## Per-Tool Isolation

Each tool name gets its own independent circuit breaker state. If tool A fails and its circuit opens, tool B continues to operate normally:

```rust,ignore
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig, CircuitState};

let cb = CircuitBreakerMiddleware::new(CircuitBreakerConfig {
    failure_threshold: 1,
    ..Default::default()
});

// After tool_a fails once (with threshold=1), only tool_a is blocked
// cb.state_for("tool_a") => CircuitState::Open
// cb.state_for("tool_b") => CircuitState::Closed
```

This ensures that one flaky tool does not shut down the entire agent. The model also gets its own isolated circuit under the internal key `__model__`.

## Model Circuit Breaking

The middleware also wraps model calls via `wrap_model_call`. This protects against scenarios where the underlying LLM API is experiencing outages:

- When the model call fails, the failure is recorded under the `__model__` circuit.
- Once the failure threshold is reached, subsequent model calls return `SynapticError::Model` immediately without making an API request.
- After the recovery timeout, a single probe call is sent to check if the model is back online.

## Usage with `create_agent`

```rust,ignore
use std::sync::Arc;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};

let options = AgentOptions {
    middleware: vec![
        Arc::new(CircuitBreakerMiddleware::new(CircuitBreakerConfig::default())),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

## How It Works

- **Lifecycle hooks:** `wrap_tool_call` and `wrap_model_call`
- Before each tool or model call, the middleware checks the circuit state for the target name.
- If the circuit is **Open** and the recovery timeout has not elapsed, the call is rejected immediately with a descriptive error.
- If the circuit is **Closed** or **HalfOpen**, the call proceeds. On success, the circuit resets to Closed with a zero failure count. On failure, the failure count increments. If the count reaches the threshold, the circuit transitions to Open.

## Testing with ScriptedChatModel

Test circuit breaker behavior without real API calls:

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use synaptic::core::{ChatResponse, Message};
use synaptic::models::ScriptedChatModel;
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};
use synaptic::graph::{create_agent, AgentOptions};

// Script the model to give a final answer (no tool calls)
let model = Arc::new(ScriptedChatModel::new(vec![
    ChatResponse {
        message: Message::ai("I can help with that."),
        usage: None,
    },
]));

let config = CircuitBreakerConfig {
    failure_threshold: 3,
    recovery_timeout: Duration::from_secs(5),
};

let options = AgentOptions {
    middleware: vec![
        Arc::new(CircuitBreakerMiddleware::new(config)),
    ],
    ..Default::default()
};

let graph = create_agent(model, vec![], options)?;
```

## Combining with Retry Middleware

The circuit breaker works well alongside `ToolRetryMiddleware`. Place the circuit breaker before the retry middleware so that retries are blocked once the circuit opens:

```rust,ignore
use std::sync::Arc;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{
    CircuitBreakerMiddleware, CircuitBreakerConfig,
    ToolRetryMiddleware,
};

let options = AgentOptions {
    middleware: vec![
        Arc::new(CircuitBreakerMiddleware::new(CircuitBreakerConfig::default())),
        Arc::new(ToolRetryMiddleware::new(3)),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

With this ordering, a tool that has tripped its circuit breaker will not waste retry attempts. If the circuit is closed, the retry middleware handles transient failures as usual.

## Inspecting Circuit State

You can query the current state of any circuit programmatically:

```rust,ignore
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig, CircuitState};

let cb = CircuitBreakerMiddleware::new(CircuitBreakerConfig::default());

let state = cb.state_for("my_tool").await;
match state {
    CircuitState::Closed => println!("Tool is healthy"),
    CircuitState::Open => println!("Tool is blocked (too many failures)"),
    CircuitState::HalfOpen => println!("Tool is being probed after recovery timeout"),
}
```

This is useful for building health-check dashboards or deciding whether to present certain tools to the agent.
