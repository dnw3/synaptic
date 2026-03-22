use async_trait::async_trait;

/// A managed lifecycle service that can be started, health-checked, and stopped.
///
/// Services differ from plugins in that they represent long-running background
/// processes (e.g., a memory provider, an embedding server, a cache daemon)
/// rather than capability registrations.
#[async_trait]
pub trait Service: Send + Sync + 'static {
    /// Unique identifier for this service instance.
    fn id(&self) -> &str;

    /// Start the service. Returns an error if startup fails.
    async fn start(&self) -> Result<(), synaptic_core::SynapticError>;

    /// Returns `true` if the service is healthy and ready to handle requests.
    async fn health_check(&self) -> bool;

    /// Gracefully stop the service.
    async fn stop(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct MockService {
        id: String,
        running: Arc<AtomicBool>,
    }

    impl MockService {
        fn new(id: impl Into<String>) -> Self {
            Self {
                id: id.into(),
                running: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    #[async_trait]
    impl Service for MockService {
        fn id(&self) -> &str {
            &self.id
        }

        async fn start(&self) -> Result<(), synaptic_core::SynapticError> {
            self.running.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn health_check(&self) -> bool {
            self.running.load(Ordering::SeqCst)
        }

        async fn stop(&self) {
            self.running.store(false, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn service_lifecycle() {
        let svc = MockService::new("mock-svc");

        assert_eq!(svc.id(), "mock-svc");

        // Not yet started — should be unhealthy
        assert!(!svc.health_check().await);

        // Start — should succeed and become healthy
        svc.start().await.expect("start should succeed");
        assert!(svc.health_check().await);

        // Stop — should become unhealthy again
        svc.stop().await;
        assert!(!svc.health_check().await);
    }
}
