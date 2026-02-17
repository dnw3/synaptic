# Build a Graph Workflow

This tutorial walks you through building a custom multi-step workflow using Synapse's LangGraph-style state graph. You will learn how to define nodes, wire them with edges, stream execution events, add conditional routing, and visualize the graph.

## Prerequisites

Add the required Synapse crates to your `Cargo.toml`:

```toml
[dependencies]
synaptic-core = { path = "../crates/synaptic-core" }
synaptic-graph = { path = "../crates/synaptic-graph" }
async-trait = "0.1"
futures = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## How State Graphs Work

A Synapse state graph is a directed graph where:

- **Nodes** are processing steps. Each node takes the current state, transforms it, and returns the new state.
- **Edges** connect nodes. Fixed edges always route to the same target; conditional edges choose the target at runtime based on the state.
- **State** is a value that flows through the graph. It carries all the data nodes need to read and write.

The lifecycle is:

```text
  START ---> node_a ---> node_b ---> node_c ---> END
              |            |            |
              v            v            v
           state_0 --> state_1 --> state_2 --> state_3
```

Each node receives the state, processes it, and passes the updated state to the next node. The graph terminates when execution reaches the `END` sentinel.

## Step 1: Define the State

The simplest built-in state is `MessageState`, which holds a `Vec<Message>`. It is suitable for most agent and chatbot workflows:

```rust
use synaptic_graph::MessageState;
use synaptic_core::Message;

let state = MessageState::with_messages(vec![
    Message::human("Hi"),
]);
```

`MessageState` implements the `State` trait, which requires a `merge()` method. When states are merged (e.g., during checkpointing or human-in-the-loop updates), `MessageState` appends the new messages to the existing list.

For custom workflows, you can implement `State` on your own types. The trait requires `Clone + Send + Sync + 'static` and a `merge` method:

```rust
use serde::{Serialize, Deserialize};
use synaptic_graph::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyState {
    counter: u32,
    results: Vec<String>,
}

impl State for MyState {
    fn merge(&mut self, other: Self) {
        self.counter += other.counter;
        self.results.extend(other.results);
    }
}
```

## Step 2: Define Nodes

A node is any type that implements the `Node<S>` trait. The trait has a single async method, `process`, which takes the state and returns the updated state:

```rust
use async_trait::async_trait;
use synaptic_core::{Message, SynapseError};
use synaptic_graph::{MessageState, Node};

struct GreetNode;

#[async_trait]
impl Node<MessageState> for GreetNode {
    async fn process(&self, mut state: MessageState) -> Result<MessageState, SynapseError> {
        state.messages.push(Message::ai("Hello! Let me help you."));
        Ok(state)
    }
}

struct ProcessNode;

#[async_trait]
impl Node<MessageState> for ProcessNode {
    async fn process(&self, mut state: MessageState) -> Result<MessageState, SynapseError> {
        state.messages.push(Message::ai("Processing your request..."));
        Ok(state)
    }
}

struct FinalizeNode;

#[async_trait]
impl Node<MessageState> for FinalizeNode {
    async fn process(&self, mut state: MessageState) -> Result<MessageState, SynapseError> {
        state.messages.push(Message::ai("Done! Here's the result."));
        Ok(state)
    }
}
```

For simpler cases, you can use `FnNode` to wrap an async closure without defining a separate struct:

```rust
use synaptic_graph::FnNode;

let greet = FnNode::new(|mut state: MessageState| async move {
    state.messages.push(Message::ai("Hello!"));
    Ok(state)
});
```

## Step 3: Build and Compile the Graph

Use `StateGraph` to wire nodes and edges into a workflow, then call `compile()` to produce an executable `CompiledGraph`:

```rust
use synaptic_graph::{StateGraph, END};

let graph = StateGraph::new()
    .add_node("greet", GreetNode)
    .add_node("process", ProcessNode)
    .add_node("finalize", FinalizeNode)
    .set_entry_point("greet")
    .add_edge("greet", "process")
    .add_edge("process", "finalize")
    .add_edge("finalize", END)
    .compile()?;
```

The builder methods are chainable:

- **`add_node(name, node)`** -- registers a named node.
- **`set_entry_point(name)`** -- designates the first node to execute.
- **`add_edge(source, target)`** -- adds a fixed edge between two nodes (use `END` as the target to terminate).
- **`compile()`** -- validates the graph and returns a `CompiledGraph`. It returns an error if the entry point is missing or if any edge references a non-existent node.

## Step 4: Invoke the Graph

Call `invoke()` with an initial state. The graph executes each node in sequence according to the edges, and returns the final state:

```rust
use synaptic_core::Message;
use synaptic_graph::MessageState;

let state = MessageState::with_messages(vec![Message::human("Hi")]);
let result = graph.invoke(state).await?;

for msg in &result.messages {
    println!("{}: {}", msg.role(), msg.content());
}
```

Output:

```text
human: Hi
ai: Hello! Let me help you.
ai: Processing your request...
ai: Done! Here's the result.
```

## Step 5: Stream Execution

For real-time feedback, use `stream()` to receive a `GraphEvent` after each node completes. Each event contains the node name and the current state snapshot:

```rust
use futures::StreamExt;
use synaptic_graph::StreamMode;

let state = MessageState::with_messages(vec![Message::human("Hi")]);
let mut stream = graph.stream(state, StreamMode::Values);

while let Some(event) = stream.next().await {
    let event = event?;
    println!("Node '{}' completed, {} messages in state",
        event.node, event.state.messages.len());
}
```

Output:

```text
Node 'greet' completed, 2 messages in state
Node 'process' completed, 3 messages in state
Node 'finalize' completed, 4 messages in state
```

`StreamMode` controls what each event contains:

- **`StreamMode::Values`** -- the event's `state` is the full accumulated state after the node ran.
- **`StreamMode::Updates`** -- the event's `state` is the state as it stands after the node, useful for observing per-node changes.

## Step 6: Add Conditional Edges

Real workflows often need branching logic. Use `add_conditional_edges` with a routing function that inspects the state and returns the name of the next node:

```rust
use std::collections::HashMap;
use synaptic_graph::{StateGraph, END};

let graph = StateGraph::new()
    .add_node("greet", GreetNode)
    .add_node("process", ProcessNode)
    .add_node("finalize", FinalizeNode)
    .set_entry_point("greet")
    .add_edge("greet", "process")
    .add_conditional_edges_with_path_map(
        "process",
        |state: &MessageState| {
            if state.messages.len() > 3 {
                "finalize".to_string()
            } else {
                "process".to_string()
            }
        },
        HashMap::from([
            ("finalize".to_string(), "finalize".to_string()),
            ("process".to_string(), "process".to_string()),
        ]),
    )
    .add_edge("finalize", END)
    .compile()?;
```

In this example, the `process` node loops back to itself until the state has more than 3 messages, at which point it routes to `finalize`.

There are two variants:

- **`add_conditional_edges(source, router_fn)`** -- the routing function returns a node name directly. Simple, but visualization tools cannot display the possible targets.
- **`add_conditional_edges_with_path_map(source, router_fn, path_map)`** -- also provides a `HashMap<String, String>` that maps labels to target node names. This enables visualization tools to show all possible routing targets.

The routing function must be `Fn(&S) -> String + Send + Sync + 'static`. It receives a reference to the current state and returns the name of the target node (or `END` to terminate).

## Step 7: Visualize the Graph

`CompiledGraph` provides several methods for visualizing the graph structure. These are useful for debugging and documentation.

### Mermaid Diagram

```rust
println!("{}", graph.draw_mermaid());
```

Produces a Mermaid flowchart that can be rendered by GitHub, GitLab, or any Mermaid-compatible viewer:

```text
graph TD
    __start__(["__start__"])
    greet["greet"]
    process["process"]
    finalize["finalize"]
    __end__(["__end__"])
    __start__ --> greet
    greet --> process
    finalize --> __end__
    process -.-> |finalize| finalize
    process -.-> |process| process
```

Fixed edges appear as solid arrows (`-->`), conditional edges as dashed arrows (`-.->`) with labels.

### ASCII Summary

```rust
println!("{}", graph.draw_ascii());
```

Produces a compact text summary:

```text
Graph:
  Nodes: finalize, greet, process
  Entry: __start__ -> greet
  Edges:
    finalize -> __end__
    greet -> process
    process -> finalize | process  [conditional]
```

### Other Formats

- **`draw_dot()`** -- produces a Graphviz DOT string, suitable for rendering with the `dot` command.
- **`draw_png(path)`** -- renders the graph as a PNG image using Graphviz (requires `dot` to be installed).
- **`draw_mermaid_png(path)`** -- renders via the mermaid.ink API (requires internet access).
- **`draw_mermaid_svg(path)`** -- renders as SVG via the mermaid.ink API.

## Complete Example

Here is the full program combining all the concepts:

```rust
use std::collections::HashMap;
use async_trait::async_trait;
use futures::StreamExt;
use synaptic_core::{Message, SynapseError};
use synaptic_graph::{MessageState, Node, StateGraph, StreamMode, END};

struct GreetNode;

#[async_trait]
impl Node<MessageState> for GreetNode {
    async fn process(&self, mut state: MessageState) -> Result<MessageState, SynapseError> {
        state.messages.push(Message::ai("Hello! Let me help you."));
        Ok(state)
    }
}

struct ProcessNode;

#[async_trait]
impl Node<MessageState> for ProcessNode {
    async fn process(&self, mut state: MessageState) -> Result<MessageState, SynapseError> {
        state.messages.push(Message::ai("Processing your request..."));
        Ok(state)
    }
}

struct FinalizeNode;

#[async_trait]
impl Node<MessageState> for FinalizeNode {
    async fn process(&self, mut state: MessageState) -> Result<MessageState, SynapseError> {
        state.messages.push(Message::ai("Done! Here's the result."));
        Ok(state)
    }
}

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    // Build the graph with a conditional loop
    let graph = StateGraph::new()
        .add_node("greet", GreetNode)
        .add_node("process", ProcessNode)
        .add_node("finalize", FinalizeNode)
        .set_entry_point("greet")
        .add_edge("greet", "process")
        .add_conditional_edges_with_path_map(
            "process",
            |state: &MessageState| {
                if state.messages.len() > 3 {
                    "finalize".to_string()
                } else {
                    "process".to_string()
                }
            },
            HashMap::from([
                ("finalize".to_string(), "finalize".to_string()),
                ("process".to_string(), "process".to_string()),
            ]),
        )
        .add_edge("finalize", END)
        .compile()?;

    // Visualize the graph
    println!("=== Graph Structure ===");
    println!("{}", graph.draw_ascii());
    println!();
    println!("=== Mermaid ===");
    println!("{}", graph.draw_mermaid());
    println!();

    // Stream execution
    println!("=== Execution ===");
    let state = MessageState::with_messages(vec![Message::human("Hi")]);
    let mut stream = graph.stream(state, StreamMode::Values);

    while let Some(event) = stream.next().await {
        let event = event?;
        let last_msg = event.state.last_message().unwrap();
        println!("[{}] {}: {}", event.node, last_msg.role(), last_msg.content());
    }

    Ok(())
}
```

Output:

```text
=== Graph Structure ===
Graph:
  Nodes: finalize, greet, process
  Entry: __start__ -> greet
  Edges:
    finalize -> __end__
    greet -> process
    process -> finalize | process  [conditional]

=== Mermaid ===
graph TD
    __start__(["__start__"])
    finalize["finalize"]
    greet["greet"]
    process["process"]
    __end__(["__end__"])
    __start__ --> greet
    finalize --> __end__
    greet --> process
    process -.-> |finalize| finalize
    process -.-> |process| process

=== Execution ===
[greet] ai: Hello! Let me help you.
[process] ai: Processing your request...
[process] ai: Processing your request...
[finalize] ai: Done! Here's the result.
```

The `process` node executes twice because on the first pass the state has only 3 messages (the human message plus greet and process outputs), so the conditional edge loops back. On the second pass it has 4 messages, which exceeds the threshold, and routing proceeds to `finalize`.

## Summary

In this tutorial you learned how to:

- Define graph state with `MessageState` or a custom `State` type
- Create nodes by implementing the `Node<S>` trait or using `FnNode`
- Build a graph with `StateGraph` using fixed and conditional edges
- Execute a graph with `invoke()` or stream it with `stream()`
- Visualize the graph with Mermaid, ASCII, DOT, and image output

## Next Steps

- [Build a ReAct Agent](react-agent.md) -- use the prebuilt `create_react_agent` helper for tool-calling agents
- [Graph How-to Guides](../how-to/graph/index.md) -- checkpointing, human-in-the-loop, streaming, and tool nodes
- [Graph Concepts](../concepts/graph.md) -- deeper look at state machines and the LangGraph execution model
