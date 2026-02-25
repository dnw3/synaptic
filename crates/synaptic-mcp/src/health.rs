//! Health monitoring for MCP server connections.
//!
//! Provides [`McpHealthMonitor`] for periodic `tools/list` probes with
//! exponential backoff on failure. Use this to detect unresponsive MCP
//! servers early and log warnings.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::MultiServerMcpClient;

/// Background health monitor for MCP server connections.
///
/// Periodically calls [`MultiServerMcpClient::connect`] (which sends
/// `tools/list` to all servers) to verify connectivity. On failure,
/// retries with exponential backoff up to `max_retries` times before
/// logging a warning and resetting.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use synaptic::mcp::{MultiServerMcpClient, McpHealthMonitor};
///
/// let client = Arc::new(MultiServerMcpClient::new(servers));
/// client.connect().await?;
///
/// let handle = McpHealthMonitor::new(client)
///     .with_interval(Duration::from_secs(60))
///     .start();
///
/// // ... later, to stop:
/// handle.stop().await;
/// ```
pub struct McpHealthMonitor {
    client: Arc<MultiServerMcpClient>,
    check_interval: Duration,
    max_retries: usize,
    backoff_base: Duration,
}

impl McpHealthMonitor {
    /// Create a new health monitor for the given MCP client.
    pub fn new(client: Arc<MultiServerMcpClient>) -> Self {
        Self {
            client,
            check_interval: Duration::from_secs(30),
            max_retries: 5,
            backoff_base: Duration::from_secs(2),
        }
    }

    /// Set the interval between health checks (default: 30s).
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Set the maximum number of retry attempts before giving up (default: 5).
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the base duration for exponential backoff (default: 2s).
    pub fn with_backoff_base(mut self, base: Duration) -> Self {
        self.backoff_base = base;
        self
    }

    /// Start the background health monitor, returning a handle to stop it.
    pub fn start(self) -> McpHealthHandle {
        let stop_signal = Arc::new(Notify::new());
        let stop_clone = stop_signal.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(self.check_interval) => {}
                    _ = stop_clone.notified() => {
                        tracing::debug!("MCP health monitor stopped");
                        return;
                    }
                }

                // Probe: try to connect (sends tools/list to all servers)
                match self.client.connect().await {
                    Ok(()) => {
                        tracing::trace!("MCP health check passed");
                    }
                    Err(e) => {
                        tracing::warn!("MCP health check failed: {}", e);

                        // Exponential backoff retries
                        let mut recovered = false;
                        for attempt in 0..self.max_retries {
                            let delay = self.backoff_base * 2u32.saturating_pow(attempt as u32);
                            tokio::select! {
                                _ = tokio::time::sleep(delay) => {}
                                _ = stop_clone.notified() => return,
                            }

                            match self.client.connect().await {
                                Ok(()) => {
                                    tracing::info!(
                                        "MCP health recovered after {} retries",
                                        attempt + 1
                                    );
                                    recovered = true;
                                    break;
                                }
                                Err(retry_err) => {
                                    tracing::warn!(
                                        "MCP health retry {}/{} failed: {}",
                                        attempt + 1,
                                        self.max_retries,
                                        retry_err
                                    );
                                }
                            }
                        }

                        if !recovered {
                            tracing::warn!(
                                "MCP health check failed after {} retries — servers may be unreachable",
                                self.max_retries
                            );
                        }
                    }
                }
            }
        });

        McpHealthHandle {
            stop_signal,
            _task: handle,
        }
    }
}

/// Handle for a running health monitor. Call [`stop`](McpHealthHandle::stop)
/// to gracefully shut down the background task.
pub struct McpHealthHandle {
    stop_signal: Arc<Notify>,
    _task: tokio::task::JoinHandle<()>,
}

impl McpHealthHandle {
    /// Signal the health monitor to stop and wait for it to finish.
    pub async fn stop(self) {
        self.stop_signal.notify_one();
        let _ = self._task.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn health_monitor_starts_and_stops() {
        let client = Arc::new(MultiServerMcpClient::new(HashMap::new()));
        let handle = McpHealthMonitor::new(client)
            .with_interval(Duration::from_millis(50))
            .start();

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(120)).await;

        // Stop should complete without hanging
        handle.stop().await;
    }
}
