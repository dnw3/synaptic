use std::sync::Arc;

use synapse_callbacks::{CompositeCallback, RecordingCallback};
use synapse_core::{CallbackHandler, RunEvent};

#[tokio::test]
async fn composite_dispatches_to_all_handlers() {
    let r1 = Arc::new(RecordingCallback::new());
    let r2 = Arc::new(RecordingCallback::new());

    let composite = CompositeCallback::new(vec![
        Arc::clone(&r1) as Arc<dyn CallbackHandler>,
        Arc::clone(&r2) as Arc<dyn CallbackHandler>,
    ]);

    composite
        .on_event(RunEvent::RunStarted {
            run_id: "r1".to_string(),
            session_id: "s1".to_string(),
        })
        .await
        .expect("dispatch event");

    let events1 = r1.events().await;
    let events2 = r2.events().await;

    assert_eq!(events1.len(), 1);
    assert_eq!(events2.len(), 1);
    assert!(matches!(events1[0], RunEvent::RunStarted { .. }));
    assert!(matches!(events2[0], RunEvent::RunStarted { .. }));
}

/// A callback that always returns an error, used to test error propagation.
struct FailingCallback;

#[async_trait::async_trait]
impl CallbackHandler for FailingCallback {
    async fn on_event(&self, _event: RunEvent) -> Result<(), synapse_core::SynapseError> {
        Err(synapse_core::SynapseError::Callback(
            "forced failure".to_string(),
        ))
    }
}

#[tokio::test]
async fn composite_propagates_error() {
    let ok_handler = Arc::new(RecordingCallback::new());
    let fail_handler = Arc::new(FailingCallback);

    let composite = CompositeCallback::new(vec![
        ok_handler as Arc<dyn CallbackHandler>,
        fail_handler as Arc<dyn CallbackHandler>,
    ]);

    let result = composite
        .on_event(RunEvent::RunStarted {
            run_id: "r1".to_string(),
            session_id: "s1".to_string(),
        })
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("forced failure"));
}
