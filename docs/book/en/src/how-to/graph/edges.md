# Edges

Edges define the flow of execution between nodes in a graph. Synapse supports two kinds of edges: **fixed edges** that always route to the same target, and **conditional edges** that route dynamically based on the current state.

## Fixed Edges

A fixed edge unconditionally routes execution from one node to another:

```rust
use synaptic_graph::{StateGraph, FnNode, MessageState, END};
use synaptic_core::Message;

let node_a = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Step A"));
    Ok(state)
});

let node_b = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Step B"));
    Ok(state)
});

let graph = StateGraph::new()
    .add_node("a", node_a)
    .add_node("b", node_b)
    .set_entry_point("a")
    .add_edge("a", "b")     // a always flows to b
    .add_edge("b", END)     // b always flows to END
    .compile()?;
```

Use the `END` constant to indicate that a node terminates the graph. Every execution path must eventually reach `END`; otherwise, the graph will hit the 100-iteration safety limit.

## Entry Point

Every graph requires an entry point -- the first node to execute:

```rust
let graph = StateGraph::new()
    .add_node("start", my_node)
    .set_entry_point("start")  // required
    // ...
```

Calling `.compile()` without setting an entry point returns an error.

## Conditional Edges

Conditional edges route execution based on a function that inspects the current state and returns the name of the next node:

```rust
use synaptic_graph::{StateGraph, FnNode, MessageState, END};
use synaptic_core::Message;

let router = FnNode::new(|state: MessageState| async move {
    Ok(state)  // routing logic is in the edge, not the node
});

let handle_greeting = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Hello!"));
    Ok(state)
});

let handle_question = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Let me look that up."));
    Ok(state)
});

let graph = StateGraph::new()
    .add_node("router", router)
    .add_node("greeting", handle_greeting)
    .add_node("question", handle_question)
    .set_entry_point("router")
    .add_conditional_edges("router", |state: &MessageState| {
        let last = state.last_message().map(|m| m.content().to_string());
        match last.as_deref() {
            Some("hi") | Some("hello") => "greeting".to_string(),
            _ => "question".to_string(),
        }
    })
    .add_edge("greeting", END)
    .add_edge("question", END)
    .compile()?;
```

The router function receives an immutable reference to the state (`&S`) and returns a `String` -- the name of the next node to execute (or `END` to terminate).

## Conditional Edges with Path Map

For graph visualization, you can provide a `path_map` that enumerates the possible routing targets. This gives visualization tools (Mermaid, DOT, ASCII) the information they need to draw all possible paths:

```rust
use std::collections::HashMap;
use synaptic_graph::{StateGraph, MessageState, END};

let graph = StateGraph::new()
    .add_node("router", router_node)
    .add_node("path_a", node_a)
    .add_node("path_b", node_b)
    .set_entry_point("router")
    .add_conditional_edges_with_path_map(
        "router",
        |state: &MessageState| {
            if state.messages.len() > 3 {
                "path_a".to_string()
            } else {
                "path_b".to_string()
            }
        },
        HashMap::from([
            ("path_a".to_string(), "path_a".to_string()),
            ("path_b".to_string(), "path_b".to_string()),
        ]),
    )
    .add_edge("path_a", END)
    .add_edge("path_b", END)
    .compile()?;
```

The `path_map` is a `HashMap<String, String>` where keys are labels and values are target node names. The compile step validates that all path map targets reference existing nodes (or `END`).

## Validation

When you call `.compile()`, the graph validates:

- An entry point is set and refers to an existing node.
- Every fixed edge source and target refers to an existing node (or `END`).
- Every conditional edge source refers to an existing node.
- All `path_map` targets refer to existing nodes (or `END`).

If any validation fails, `compile()` returns a `SynapseError::Graph` with a descriptive message.
