use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::SynapticError;
use tokio::sync::RwLock;

use crate::{
    AgentMiddleware, ModelCaller, ModelRequest, ModelResponse, ToolCallRequest, ToolCaller,
};

/// Circuit breaker state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests flow through.
    Closed,
    /// Failures exceeded threshold — requests are rejected immediately.
    Open,
    /// After recovery timeout — allows a single probe request.
    HalfOpen,
}

/// Per-target circuit state.
#[derive(Debug)]
struct CircuitTracker {
    state: CircuitState,
    failure_count: usize,
    last_failure: Option<Instant>,
}

impl CircuitTracker {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
        }
    }
}

/// Configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: usize,
    /// Time to wait before transitioning from Open to HalfOpen.
    pub recovery_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
        }
    }
}

/// Middleware that implements the circuit breaker pattern for tool calls.
///
/// Tracks failures per tool name and opens the circuit when failures
/// exceed the configured threshold. After the recovery timeout, a single
/// probe request is allowed through (half-open). If it succeeds, the
/// circuit closes; if it fails, the circuit reopens.
pub struct CircuitBreakerMiddleware {
    config: CircuitBreakerConfig,
    circuits: Arc<RwLock<HashMap<String, CircuitTracker>>>,
}

impl CircuitBreakerMiddleware {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            circuits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the current state for a given tool name.
    pub async fn state_for(&self, tool_name: &str) -> CircuitState {
        let circuits = self.circuits.read().await;
        circuits
            .get(tool_name)
            .map(|t| {
                if t.state == CircuitState::Open {
                    // Check if recovery timeout has elapsed
                    if let Some(last_failure) = t.last_failure {
                        if last_failure.elapsed() >= self.config.recovery_timeout {
                            return CircuitState::HalfOpen;
                        }
                    }
                }
                t.state
            })
            .unwrap_or(CircuitState::Closed)
    }

    async fn record_success(&self, tool_name: &str) {
        let mut circuits = self.circuits.write().await;
        let tracker = circuits
            .entry(tool_name.to_string())
            .or_insert_with(CircuitTracker::new);
        tracker.state = CircuitState::Closed;
        tracker.failure_count = 0;
    }

    async fn record_failure(&self, tool_name: &str) {
        let mut circuits = self.circuits.write().await;
        let tracker = circuits
            .entry(tool_name.to_string())
            .or_insert_with(CircuitTracker::new);
        tracker.failure_count += 1;
        tracker.last_failure = Some(Instant::now());
        if tracker.failure_count >= self.config.failure_threshold {
            tracker.state = CircuitState::Open;
        }
    }
}

#[async_trait]
impl AgentMiddleware for CircuitBreakerMiddleware {
    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        let tool_name = &request.call.name;
        let state = self.state_for(tool_name).await;

        match state {
            CircuitState::Open => Err(SynapticError::Tool(format!(
                "circuit breaker open for tool '{}' — too many consecutive failures",
                tool_name
            ))),
            CircuitState::HalfOpen | CircuitState::Closed => {
                match next.call(request.clone()).await {
                    Ok(result) => {
                        self.record_success(tool_name).await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.record_failure(tool_name).await;
                        Err(e)
                    }
                }
            }
        }
    }

    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        let state = self.state_for("__model__").await;

        match state {
            CircuitState::Open => Err(SynapticError::Model(
                "circuit breaker open for model — too many consecutive failures".to_string(),
            )),
            CircuitState::HalfOpen | CircuitState::Closed => match next.call(request).await {
                Ok(result) => {
                    self.record_success("__model__").await;
                    Ok(result)
                }
                Err(e) => {
                    self.record_failure("__model__").await;
                    Err(e)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn circuit_starts_closed() {
        let cb = CircuitBreakerMiddleware::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state_for("test_tool").await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn circuit_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
        };
        let cb = CircuitBreakerMiddleware::new(config);

        cb.record_failure("tool_a").await;
        cb.record_failure("tool_a").await;
        assert_eq!(cb.state_for("tool_a").await, CircuitState::Closed);

        cb.record_failure("tool_a").await;
        assert_eq!(cb.state_for("tool_a").await, CircuitState::Open);
    }

    #[tokio::test]
    async fn circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: Duration::from_millis(10),
        };
        let cb = CircuitBreakerMiddleware::new(config);

        cb.record_failure("tool_a").await;
        assert_eq!(cb.state_for("tool_a").await, CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(cb.state_for("tool_a").await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn success_resets_circuit() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_secs(60),
        };
        let cb = CircuitBreakerMiddleware::new(config);

        cb.record_failure("tool_a").await;
        cb.record_success("tool_a").await;
        assert_eq!(cb.state_for("tool_a").await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn per_tool_isolation() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: Duration::from_secs(60),
        };
        let cb = CircuitBreakerMiddleware::new(config);

        cb.record_failure("tool_a").await;
        assert_eq!(cb.state_for("tool_a").await, CircuitState::Open);
        assert_eq!(cb.state_for("tool_b").await, CircuitState::Closed);
    }
}
