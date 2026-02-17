# Human-in-the-Loop

Human-in-the-loop (HITL) allows you to pause graph execution at specific nodes, giving a human the opportunity to review, approve, or modify the state before the graph continues. This is built on top of [checkpointing](checkpointing.md) -- interrupts save a checkpoint and return an error, and resuming loads the checkpoint and continues.

## Interrupt Before and After

The `StateGraph` builder provides two interrupt modes:

- **`interrupt_before(nodes)`** -- pause execution **before** the named nodes run.
- **`interrupt_after(nodes)`** -- pause execution **after** the named nodes run.

Both require a checkpointer to be attached; otherwise the graph cannot persist the state for later resumption.

## Example: Approval Before Tool Execution

A common pattern is to interrupt before a tool execution node so a human can review the tool calls the agent proposed:

```rust
use synapse_graph::{StateGraph, FnNode, MessageState, MemorySaver, CheckpointConfig, END};
use synapse_core::Message;
use std::sync::Arc;

let agent_node = FnNode::new(|mut state: MessageState| async move {
    // Agent decides which tool to call
    state.messages.push(Message::ai("I want to call the delete_file tool."));
    Ok(state)
});

let tool_node = FnNode::new(|mut state: MessageState| async move {
    // Execute the tool
    state.messages.push(Message::tool("File deleted.", "call-1"));
    Ok(state)
});

let graph = StateGraph::new()
    .add_node("agent", agent_node)
    .add_node("tools", tool_node)
    .set_entry_point("agent")
    .add_edge("agent", "tools")
    .add_edge("tools", END)
    // Pause before the tools node executes
    .interrupt_before(vec!["tools".to_string()])
    .compile()?
    .with_checkpointer(Arc::new(MemorySaver::new()));

let config = CheckpointConfig::new("thread-1");
let initial = MessageState::with_messages(vec![Message::human("Delete old logs")]);
```

## Step 1: First Invocation -- Interrupt

The first `invoke_with_config()` runs the `agent` node, then stops before `tools`:

```rust
let result = graph.invoke_with_config(initial, Some(config.clone())).await;

// Returns Err because the graph is interrupted
match result {
    Err(e) => println!("Interrupted: {e}"),
    // e.g. "interrupted before node 'tools'"
    Ok(_) => unreachable!(),
}
```

At this point, the checkpointer has saved the state after `agent` ran, with `tools` as the next node.

## Step 2: Human Review

The human can inspect the saved state to review what the agent proposed:

```rust
if let Some(state) = graph.get_state(&config).await? {
    for msg in &state.messages {
        println!("[{}] {}", msg.role(), msg.content());
    }
}
```

## Step 3: Update State (Optional)

If the human wants to modify the state before resuming -- for example, to add an approval message or to change the tool call -- use `update_state()`:

```rust
let approval = MessageState::with_messages(vec![
    Message::human("Approved -- go ahead and delete."),
]);

graph.update_state(&config, approval).await?;
```

`update_state()` loads the current checkpoint, calls `State::merge()` with the provided update, and saves the merged result back to the checkpointer.

## Step 4: Resume Execution

Resume the graph by calling `invoke_with_config()` again with the same config and a default (empty) state. The graph loads the checkpoint and continues from the interrupted node:

```rust
let result = graph
    .invoke_with_config(MessageState::default(), Some(config))
    .await?;

// The graph executed "tools" and reached END
println!("Final messages: {}", result.messages.len());
```

## `interrupt_after`

`interrupt_after` works the same way, but the specified node runs **before** the interrupt. This is useful when you want to see the node's output before deciding whether to continue:

```rust
let graph = StateGraph::new()
    .add_node("agent", agent_node)
    .add_node("tools", tool_node)
    .set_entry_point("agent")
    .add_edge("agent", "tools")
    .add_edge("tools", END)
    // Interrupt after the agent node runs (to review its output)
    .interrupt_after(vec!["agent".to_string()])
    .compile()?
    .with_checkpointer(Arc::new(MemorySaver::new()));
```

## Notes

- Interrupts require a checkpointer. Without one, the graph cannot save state for resumption.
- The interrupt error is a `SynapseError::Graph` with a message like `"interrupted before node 'tools'"` or `"interrupted after node 'agent'"`.
- You can interrupt at multiple nodes by passing multiple names to `interrupt_before()` or `interrupt_after()`.
- You can combine `interrupt_before` and `interrupt_after` on different nodes in the same graph.
