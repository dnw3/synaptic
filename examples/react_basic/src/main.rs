use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use synapse::core::{ChatModel, ChatRequest, ChatResponse, Message, SynapseError, Tool, ToolCall};
use synapse::graph::{create_react_agent, MessageState};

struct DemoModel;

#[async_trait]
impl ChatModel for DemoModel {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError> {
        let has_tool_output = request.messages.iter().any(|m| m.is_tool());
        if !has_tool_output {
            Ok(ChatResponse {
                message: Message::ai_with_tool_calls(
                    "I will use a tool to calculate this.",
                    vec![ToolCall {
                        id: "call-1".to_string(),
                        name: "add".to_string(),
                        arguments: json!({ "a": 7, "b": 5 }),
                    }],
                ),
                usage: None,
            })
        } else {
            Ok(ChatResponse {
                message: Message::ai("The result is 12."),
                usage: None,
            })
        }
    }
}

struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &'static str {
        "add"
    }

    fn description(&self) -> &'static str {
        "Adds two numbers."
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, SynapseError> {
        let a = args["a"].as_i64().unwrap_or_default();
        let b = args["b"].as_i64().unwrap_or_default();
        Ok(json!({ "value": a + b }))
    }
}

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    let model = Arc::new(DemoModel);
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(AddTool)];

    let graph = create_react_agent(model, tools)?;

    let initial_state = MessageState {
        messages: vec![Message::human("What is 7 + 5?")],
    };

    let result = graph.invoke(initial_state).await?;
    let last = result.last_message().unwrap();
    println!("agent answer: {}", last.content());
    println!("message_count: {}", result.messages.len());
    Ok(())
}
