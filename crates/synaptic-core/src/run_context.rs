/// Per-run execution context, passed at run time (not build time).
///
/// Carries data specific to a single agent invocation:
/// streaming output callbacks, cancellation tokens, etc.
#[derive(Default, Clone)]
pub struct RunContext {
    /// Cancellation signal — set to `true` to abort execution.
    pub cancel_token: Option<tokio::sync::watch::Receiver<bool>>,
}
