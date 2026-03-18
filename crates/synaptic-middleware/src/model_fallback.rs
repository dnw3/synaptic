#![allow(deprecated)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use synaptic_core::{ChatModel, SynapticError};
use tokio::sync::Mutex;

use crate::{AgentMiddleware, BaseChatModelCaller, ModelCaller, ModelRequest, ModelResponse};

/// Error classification for failover decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorKind {
    /// Transient errors (429 rate limit, 5xx server errors) — short cooldown.
    Transient,
    /// Persistent errors (401 unauthorized, 403 forbidden) — long cooldown.
    Persistent,
}

/// Tracks the health state of a fallback model/key.
struct KeyState {
    /// When the last error occurred.
    last_error: Option<Instant>,
    /// Classification of the last error.
    error_kind: Option<ErrorKind>,
    /// Cooldown expiry time.
    cooldown_until: Option<Instant>,
    /// Consecutive successful calls (used for MRU ordering).
    success_count: u64,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            last_error: None,
            error_kind: None,
            cooldown_until: None,
            success_count: 0,
        }
    }
}

impl KeyState {
    fn is_in_cooldown(&self) -> bool {
        self.cooldown_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    fn record_success(&mut self) {
        self.success_count += 1;
        self.last_error = None;
        self.error_kind = None;
        self.cooldown_until = None;
    }

    fn record_error(&mut self, kind: ErrorKind) {
        self.last_error = Some(Instant::now());
        self.error_kind = Some(kind);
        self.success_count = 0;

        let cooldown = match kind {
            ErrorKind::Transient => Duration::from_secs(30),
            ErrorKind::Persistent => Duration::from_secs(300),
        };
        self.cooldown_until = Some(Instant::now() + cooldown);
    }
}

/// Classify a SynapticError into transient vs persistent.
fn classify_error(err: &SynapticError) -> ErrorKind {
    match err {
        SynapticError::RateLimit(_) => ErrorKind::Transient,
        SynapticError::Model(msg) => {
            let msg_lower = msg.to_lowercase();
            if msg_lower.contains("401")
                || msg_lower.contains("403")
                || msg_lower.contains("unauthorized")
                || msg_lower.contains("forbidden")
                || msg_lower.contains("invalid api key")
            {
                ErrorKind::Persistent
            } else if msg_lower.contains("429")
                || msg_lower.contains("500")
                || msg_lower.contains("502")
                || msg_lower.contains("503")
                || msg_lower.contains("504")
                || msg_lower.contains("rate limit")
                || msg_lower.contains("server error")
            {
                ErrorKind::Transient
            } else {
                ErrorKind::Transient // default to transient for unknown errors
            }
        }
        _ => ErrorKind::Transient,
    }
}

/// Falls back to alternative models when the primary model fails.
///
/// Enhanced features:
/// - **Error classification**: Distinguishes transient (429/5xx) from persistent (401/403) errors
/// - **Cooldown tracking**: Skips models in cooldown to avoid repeated failures
/// - **MRU ordering**: Prefers recently-successful fallbacks
#[deprecated(note = "Use EventSubscriber instead. This will be removed in a future version.")]
pub struct ModelFallbackMiddleware {
    fallbacks: Vec<Arc<dyn ChatModel>>,
    /// Per-fallback health state (index 0 = primary, 1.. = fallbacks).
    states: Mutex<Vec<KeyState>>,
}

impl ModelFallbackMiddleware {
    pub fn new(fallbacks: Vec<Arc<dyn ChatModel>>) -> Self {
        let count = fallbacks.len() + 1; // +1 for primary
        let states = (0..count).map(|_| KeyState::default()).collect();
        Self {
            fallbacks,
            states: Mutex::new(states),
        }
    }
}

#[allow(deprecated)]
#[async_trait]
impl AgentMiddleware for ModelFallbackMiddleware {
    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        // Try primary (index 0)
        let primary_in_cooldown = self.states.lock().await[0].is_in_cooldown();

        if !primary_in_cooldown {
            match next.call(request.clone()).await {
                Ok(resp) => {
                    self.states.lock().await[0].record_success();
                    return Ok(resp);
                }
                Err(err) => {
                    let kind = classify_error(&err);
                    self.states.lock().await[0].record_error(kind);

                    if self.fallbacks.is_empty() {
                        return Err(err);
                    }
                    // Fall through to try fallbacks
                }
            }
        }

        // Build priority order: sort fallbacks by success_count (MRU first),
        // excluding those in cooldown.
        let indices: Vec<usize> = {
            let states = self.states.lock().await;
            let mut candidates: Vec<(usize, u64)> = (0..self.fallbacks.len())
                .filter(|i| !states[i + 1].is_in_cooldown())
                .map(|i| (i, states[i + 1].success_count))
                .collect();
            // Sort by success_count descending (MRU first)
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            candidates.into_iter().map(|(i, _)| i).collect()
        };

        for i in indices {
            let caller = BaseChatModelCaller::new(self.fallbacks[i].clone());
            match caller.call(request.clone()).await {
                Ok(resp) => {
                    self.states.lock().await[i + 1].record_success();
                    return Ok(resp);
                }
                Err(err) => {
                    let kind = classify_error(&err);
                    self.states.lock().await[i + 1].record_error(kind);
                    continue;
                }
            }
        }

        // All models failed or in cooldown — try any cooldown model as last resort
        for (i, fallback) in self.fallbacks.iter().enumerate() {
            if !self.states.lock().await[i + 1].is_in_cooldown() {
                continue; // already tried
            }
            let caller = BaseChatModelCaller::new(fallback.clone());
            match caller.call(request.clone()).await {
                Ok(resp) => {
                    self.states.lock().await[i + 1].record_success();
                    return Ok(resp);
                }
                Err(_) => continue,
            }
        }

        Err(SynapticError::Model(
            "all models failed (primary + fallbacks)".to_string(),
        ))
    }
}
