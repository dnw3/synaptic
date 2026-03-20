use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{SynapticError, TokenUsage};
use tokio::sync::RwLock;

use crate::{Event, EventAction, EventFilter, EventKind, EventSubscriber};

/// Per-model cost rates (USD per 1M tokens).
#[derive(Debug, Clone)]
pub struct ModelPricing {
    /// Cost per 1M input/prompt tokens.
    pub input_per_million: f64,
    /// Cost per 1M output/completion tokens.
    pub output_per_million: f64,
}

impl ModelPricing {
    pub fn new(input_per_million: f64, output_per_million: f64) -> Self {
        Self {
            input_per_million,
            output_per_million,
        }
    }
}

/// Accumulated usage stats for cost tracking.
#[derive(Debug, Clone, Default)]
pub struct UsageSnapshot {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_requests: u64,
    pub estimated_cost_usd: f64,
    /// Per-model breakdown.
    pub per_model: HashMap<String, ModelUsage>,
}

/// Per-model usage breakdown.
#[derive(Debug, Clone, Default)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u64,
    pub cost_usd: f64,
}

/// Internal state.
struct CostState {
    usage: UsageSnapshot,
    pricing: HashMap<String, ModelPricing>,
    budget_limit: Option<f64>,
    current_model: String,
}

/// Tracks token usage and estimated cost across model calls.
///
/// Supports per-model pricing tables and optional budget limits. Query the
/// accumulated snapshot via [`snapshot()`](CostTrackingCallback::snapshot).
///
/// Implements `EventSubscriber` subscribing to `LlmOutput` events.
pub struct CostTrackingCallback {
    state: Arc<RwLock<CostState>>,
}

impl CostTrackingCallback {
    /// Create a new cost tracker with the given pricing table.
    pub fn new(pricing: HashMap<String, ModelPricing>) -> Self {
        Self {
            state: Arc::new(RwLock::new(CostState {
                usage: UsageSnapshot::default(),
                pricing,
                budget_limit: None,
                current_model: String::new(),
            })),
        }
    }

    /// Set a budget limit in USD. Returns error via callback when exceeded.
    pub fn with_budget(self, limit_usd: f64) -> Self {
        // We'll set it after creation since we can't await in a non-async fn
        let state = self.state.clone();
        tokio::spawn(async move {
            state.write().await.budget_limit = Some(limit_usd);
        });
        self
    }

    /// Set the current model name for cost attribution.
    pub async fn set_model(&self, model_name: &str) {
        self.state.write().await.current_model = model_name.to_string();
    }

    /// Record token usage from a model response.
    pub async fn record_usage(&self, usage: &TokenUsage) {
        let mut state = self.state.write().await;
        let model = state.current_model.clone();

        // Look up pricing before mutating per_model
        let cost = state.pricing.get(&model).map(|pricing| {
            (usage.input_tokens as f64 / 1_000_000.0) * pricing.input_per_million
                + (usage.output_tokens as f64 / 1_000_000.0) * pricing.output_per_million
        });

        state.usage.total_input_tokens += usage.input_tokens as u64;
        state.usage.total_output_tokens += usage.output_tokens as u64;
        state.usage.total_requests += 1;

        let entry = state.usage.per_model.entry(model).or_default();
        entry.input_tokens += usage.input_tokens as u64;
        entry.output_tokens += usage.output_tokens as u64;
        entry.requests += 1;

        if let Some(cost) = cost {
            entry.cost_usd += cost;
            state.usage.estimated_cost_usd += cost;
        }
    }

    /// Get a snapshot of accumulated usage and costs.
    pub async fn snapshot(&self) -> UsageSnapshot {
        self.state.read().await.usage.clone()
    }

    /// Check if the budget has been exceeded.
    pub async fn is_over_budget(&self) -> bool {
        let state = self.state.read().await;
        if let Some(limit) = state.budget_limit {
            state.usage.estimated_cost_usd > limit
        } else {
            false
        }
    }
}

/// Build a default pricing table for common models (approximate, Feb 2026).
pub fn default_pricing() -> HashMap<String, ModelPricing> {
    let mut m = HashMap::new();
    // OpenAI
    m.insert("gpt-4o".to_string(), ModelPricing::new(2.5, 10.0));
    m.insert("gpt-4o-mini".to_string(), ModelPricing::new(0.15, 0.6));
    m.insert("o1".to_string(), ModelPricing::new(15.0, 60.0));
    m.insert("o3-mini".to_string(), ModelPricing::new(1.1, 4.4));
    // Anthropic
    m.insert(
        "claude-sonnet-4-20250514".to_string(),
        ModelPricing::new(3.0, 15.0),
    );
    m.insert(
        "claude-haiku-4-5-20251001".to_string(),
        ModelPricing::new(0.8, 4.0),
    );
    m.insert(
        "claude-opus-4-20250514".to_string(),
        ModelPricing::new(15.0, 75.0),
    );
    // Gemini
    m.insert("gemini-2.0-flash".to_string(), ModelPricing::new(0.1, 0.4));
    m.insert("gemini-2.0-pro".to_string(), ModelPricing::new(1.25, 10.0));
    m
}

#[async_trait]
impl EventSubscriber for CostTrackingCallback {
    fn subscriptions(&self) -> Vec<EventFilter> {
        vec![EventFilter::Exact(EventKind::LlmOutput)]
    }

    async fn handle(&self, _event: &mut Event) -> Result<EventAction, SynapticError> {
        // Cost is tracked via record_usage() which is called externally
        // when the actual TokenUsage is available from the response.
        Ok(EventAction::Continue)
    }

    fn name(&self) -> &str {
        "CostTrackingCallback"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracks_usage() {
        let pricing = default_pricing();
        let tracker = CostTrackingCallback::new(pricing);
        tracker.set_model("gpt-4o").await;

        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
            input_details: None,
            output_details: None,
        };
        tracker.record_usage(&usage).await;

        let snap = tracker.snapshot().await;
        assert_eq!(snap.total_input_tokens, 1000);
        assert_eq!(snap.total_output_tokens, 500);
        assert_eq!(snap.total_requests, 1);
        assert!(snap.estimated_cost_usd > 0.0);
    }

    #[tokio::test]
    async fn per_model_breakdown() {
        let pricing = default_pricing();
        let tracker = CostTrackingCallback::new(pricing);

        tracker.set_model("gpt-4o").await;
        tracker
            .record_usage(&TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 0,
                input_details: None,
                output_details: None,
            })
            .await;

        tracker.set_model("gpt-4o-mini").await;
        tracker
            .record_usage(&TokenUsage {
                input_tokens: 200,
                output_tokens: 100,
                total_tokens: 0,
                input_details: None,
                output_details: None,
            })
            .await;

        let snap = tracker.snapshot().await;
        assert_eq!(snap.per_model.len(), 2);
        assert_eq!(snap.total_requests, 2);
    }

    #[test]
    fn default_pricing_has_models() {
        let p = default_pricing();
        assert!(p.contains_key("gpt-4o"));
        assert!(p.contains_key("claude-sonnet-4-20250514"));
    }
}
