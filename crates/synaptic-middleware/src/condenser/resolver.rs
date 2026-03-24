use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use synaptic_core::SynapticError;

/// Resolves the context window size for a given model/provider pair.
#[async_trait]
pub trait ContextWindowResolver: Send + Sync {
    /// Return the context window size in tokens for the given model and provider.
    fn resolve(&self, model: &str, provider: &str) -> usize;

    /// Optionally discover model capabilities (e.g. from a remote API).
    /// Default implementation is a no-op.
    async fn discover(&self) -> Result<(), SynapticError> {
        Ok(())
    }
}

/// A simple resolver that caches known model context windows and falls back
/// to a configurable default.
pub struct DefaultContextWindowResolver {
    cache: RwLock<HashMap<String, usize>>,
    fallback: usize,
}

impl DefaultContextWindowResolver {
    /// Create a new resolver with the given fallback context window size.
    pub fn new(fallback: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            fallback,
        }
    }

    /// Register a known context window for a model/provider pair.
    pub fn register(&self, model: &str, provider: &str, context_window: usize) {
        let key = Self::cache_key(model, provider);
        self.cache
            .write()
            .expect("resolver cache poisoned")
            .insert(key, context_window);
    }

    fn cache_key(model: &str, provider: &str) -> String {
        format!("{provider}:{model}")
    }
}

#[async_trait]
impl ContextWindowResolver for DefaultContextWindowResolver {
    fn resolve(&self, model: &str, provider: &str) -> usize {
        let key = Self::cache_key(model, provider);
        self.cache
            .read()
            .expect("resolver cache poisoned")
            .get(&key)
            .copied()
            .unwrap_or(self.fallback)
    }
}
