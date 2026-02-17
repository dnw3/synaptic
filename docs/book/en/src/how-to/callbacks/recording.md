# Recording Callback

`RecordingCallback` captures every `RunEvent` in an in-memory list. This is useful for testing agent behavior, debugging execution flow, and building audit logs.

## Usage

```rust
use synapse_callbacks::RecordingCallback;
use synapse_core::RunEvent;

let callback = RecordingCallback::new();

// ... pass the callback to an agent or use it manually ...

// After the run, inspect all recorded events
let events = callback.events().await;
for event in &events {
    match event {
        RunEvent::RunStarted { run_id, session_id } => {
            println!("Run started: run_id={run_id}, session={session_id}");
        }
        RunEvent::RunStep { run_id, step } => {
            println!("Step {step} in run {run_id}");
        }
        RunEvent::LlmCalled { run_id, message_count } => {
            println!("LLM called with {message_count} messages (run {run_id})");
        }
        RunEvent::ToolCalled { run_id, tool_name } => {
            println!("Tool '{tool_name}' called (run {run_id})");
        }
        RunEvent::RunFinished { run_id, output } => {
            println!("Run {run_id} finished: {output}");
        }
        RunEvent::RunFailed { run_id, error } => {
            println!("Run {run_id} failed: {error}");
        }
    }
}
```

## How It Works

`RecordingCallback` stores events in an `Arc<RwLock<Vec<RunEvent>>>`. Each call to `on_event()` appends the event to the list. The `events()` method returns a clone of the full event list.

Because it uses `Arc`, the callback can be cloned and shared across tasks. All clones refer to the same event storage.

## Testing Example

`RecordingCallback` is particularly useful in tests to verify that an agent followed the expected execution path:

```rust
#[tokio::test]
async fn test_agent_calls_tool() {
    let callback = RecordingCallback::new();

    // ... run the agent with this callback ...

    let events = callback.events().await;

    // Verify the agent called the expected tool
    let tool_events: Vec<_> = events.iter()
        .filter_map(|e| match e {
            RunEvent::ToolCalled { tool_name, .. } => Some(tool_name.clone()),
            _ => None,
        })
        .collect();

    assert!(tool_events.contains(&"calculator".to_string()));
}
```

## Thread Safety

`RecordingCallback` is `Clone`, `Send`, and `Sync`. You can safely share it across async tasks and inspect events from any task that holds a reference.
