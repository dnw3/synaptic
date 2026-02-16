# react_basic

Demonstrates a ReAct (Reason + Act) agent using `create_react_agent` from the graph crate. The agent receives a question, decides to call a tool, processes the tool result, and returns a final answer.

## What it does

1. Defines a `DemoModel` (scripted `ChatModel`) that emits a tool call on the first turn and a text answer after seeing tool output
2. Defines an `AddTool` that adds two numbers
3. Builds a ReAct agent graph with `create_react_agent(model, tools)`
4. Invokes the graph with `"What is 7 + 5?"` as the initial message
5. The agent calls the `add` tool, receives `{"value": 12}`, then responds `"The result is 12."`

## Run

```bash
cargo run -p react_basic
```

## Expected output

```
agent answer: The result is 12.
message_count: 4
```

The 4 messages are: human question, AI tool call, tool result, AI final answer.

## Key concepts

- **`ChatModel` trait** — implement `chat(request) -> ChatResponse` for any LLM backend
- **`Tool` trait** — implement `name()`, `description()`, `call(args)` for agent-callable tools
- **`create_react_agent(model, tools)`** — builds a compiled `StateGraph` that runs the ReAct loop automatically
- **`MessageState`** — the graph state containing the conversation message list
- **ReAct loop** — the agent alternates between LLM reasoning and tool execution until no more tool calls are requested
