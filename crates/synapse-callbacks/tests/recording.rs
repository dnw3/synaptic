use synapse_callbacks::RecordingCallback;
use synapse_core::{CallbackHandler, RunEvent};

#[tokio::test]
async fn records_events_in_order() {
    let callback = RecordingCallback::new();

    callback
        .on_event(RunEvent::RunStarted {
            run_id: "run-1".to_string(),
            session_id: "session-1".to_string(),
        })
        .await
        .expect("start event");

    callback
        .on_event(RunEvent::RunFinished {
            run_id: "run-1".to_string(),
            output: "done".to_string(),
        })
        .await
        .expect("finish event");

    let events = callback.events().await;
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], RunEvent::RunStarted { .. }));
    assert!(matches!(events[1], RunEvent::RunFinished { .. }));
}
