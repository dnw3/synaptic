# Graph Streaming

Instead of waiting for the entire graph to finish, you can **stream** execution and receive a `GraphEvent` after each node completes. This is useful for progress reporting, real-time UIs, and debugging.

## `stream()` and `StreamMode`

The `stream()` method on `CompiledGraph` returns a `GraphStream` -- a `Pin<Box<dyn Stream>>` that yields `Result<GraphEvent<S>, SynapseError>` values:

```rust
use synapse_graph::{StateGraph, FnNode, MessageState, StreamMode, GraphEvent, END};
use synapse_core::Message;
use futures::StreamExt;

let step_a = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Step A done"));
    Ok(state)
});

let step_b = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Step B done"));
    Ok(state)
});

let graph = StateGraph::new()
    .add_node("a", step_a)
    .add_node("b", step_b)
    .set_entry_point("a")
    .add_edge("a", "b")
    .add_edge("b", END)
    .compile()?;

let initial = MessageState::with_messages(vec![Message::human("Start")]);

let mut stream = graph.stream(initial, StreamMode::Values);
while let Some(event) = stream.next().await {
    let event: GraphEvent<MessageState> = event?;
    println!(
        "Node '{}' completed -- {} messages in state",
        event.node,
        event.state.messages.len()
    );
}
// Output:
//   Node 'a' completed -- 2 messages in state
//   Node 'b' completed -- 3 messages in state
```

## `GraphEvent`

Each event contains:

| Field | Type | Description |
|-------|------|-------------|
| `node` | `String` | The name of the node that just executed |
| `state` | `S` | The state snapshot after the node ran |

## Stream Modes

The `StreamMode` enum controls what the `state` field contains:

| Mode | Behavior |
|------|----------|
| `StreamMode::Values` | Each event contains the **full accumulated state** after the node |
| `StreamMode::Updates` | Each event contains the **post-node state** (useful for seeing per-node contributions) |

## Streaming with Checkpoints

You can combine streaming with checkpointing using `stream_with_config()`:

```rust
use synapse_graph::{MemorySaver, CheckpointConfig, StreamMode};
use std::sync::Arc;

let checkpointer = Arc::new(MemorySaver::new());
let graph = graph.with_checkpointer(checkpointer);

let config = CheckpointConfig::new("thread-1");

let mut stream = graph.stream_with_config(
    initial_state,
    StreamMode::Values,
    Some(config),
);

while let Some(event) = stream.next().await {
    let event = event?;
    println!("Node: {}", event.node);
}
```

Checkpoints are saved after each node during streaming, just as they are during `invoke()`. If the graph is interrupted (via `interrupt_before` or `interrupt_after`), the stream yields the interrupt error and terminates.

## Error Handling

The stream yields `Result` values. If a node returns an error, the stream yields that error and terminates. Consuming code should handle both successful events and errors:

```rust
while let Some(result) = stream.next().await {
    match result {
        Ok(event) => println!("Node '{}' succeeded", event.node),
        Err(e) => {
            eprintln!("Graph error: {e}");
            break;
        }
    }
}
```
