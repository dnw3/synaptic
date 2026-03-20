use std::any::Any;
use std::sync::Arc;

/// Per-run execution context, passed at run time (not build time).
///
/// Carries data specific to a single agent invocation:
/// streaming output callbacks, cancellation tokens, etc.
#[derive(Default, Clone)]
pub struct RunContext {
    /// Cancellation signal — set to `true` to abort execution.
    pub cancel_token: Option<tokio::sync::watch::Receiver<bool>>,
    /// Opaque streaming output handle.
    ///
    /// Holds an `Arc<dyn StreamingOutput>` (from `synaptic-graph`), stored as
    /// `Arc<dyn Any + Send + Sync>` to avoid a circular dependency.
    /// Use [`Self::with_streaming_output`] / [`Self::streaming_output`] for
    /// typed access.
    pub streaming_output: Option<Arc<dyn Any + Send + Sync>>,
}

impl RunContext {
    /// Attach a streaming output handle.
    ///
    /// The concrete type `T` is typically `Arc<dyn StreamingOutput>`.
    /// Callers recover it via [`streaming_output`](Self::streaming_output).
    pub fn with_streaming_output<T: Send + Sync + 'static>(mut self, output: Arc<T>) -> Self {
        self.streaming_output = Some(output as Arc<dyn Any + Send + Sync>);
        self
    }

    /// Downcast the opaque streaming output to the expected concrete wrapper.
    pub fn streaming_output<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.streaming_output
            .as_ref()
            .and_then(|any| any.clone().downcast::<T>().ok())
    }
}
