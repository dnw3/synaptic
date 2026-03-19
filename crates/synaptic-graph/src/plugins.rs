//! Unified plugin registration for agent interceptors and observers.

use std::sync::Arc;
use synaptic_middleware::{Interceptor, InterceptorChain};

/// Unified registration point for agent interceptors and observers.
///
/// Interceptors are synchronous chain-based (can modify/abort).
/// Observers are async fire-and-forget (via EventBus, registered separately).
pub struct AgentPlugins {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl AgentPlugins {
    pub fn new() -> Self {
        Self {
            interceptors: vec![],
        }
    }

    pub fn with_interceptor(mut self, i: Arc<dyn Interceptor>) -> Self {
        self.interceptors.push(i);
        self
    }

    pub fn add_interceptor(&mut self, i: Arc<dyn Interceptor>) {
        self.interceptors.push(i);
    }

    pub fn interceptor_chain(&self) -> InterceptorChain {
        InterceptorChain::new(self.interceptors.clone())
    }

    pub fn interceptors(&self) -> &[Arc<dyn Interceptor>] {
        &self.interceptors
    }
}

impl Default for AgentPlugins {
    fn default() -> Self {
        Self::new()
    }
}
