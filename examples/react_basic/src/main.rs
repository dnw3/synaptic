use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use synapse_agents::{AgentConfig, ReActAgentExecutor};
use synapse_callbacks::RecordingCallback;
use synapse_core::{
    Agent, ChatModel, ChatRequest, ChatResponse, Message, SynapseError, Tool, ToolCall,
};
use synapse_memory::InMemoryStore;
use synapse_tools::{SerialToolExecutor, ToolRegistry};

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
    let memory = Arc::new(InMemoryStore::new());
    let callbacks = Arc::new(RecordingCallback::new());

    let registry = ToolRegistry::new();
    registry.register(Arc::new(AddTool))?;
    let tools = Arc::new(SerialToolExecutor::new(registry));

    let agent = ReActAgentExecutor::new(
        model,
        tools,
        memory,
        callbacks.clone(),
        AgentConfig {
            system_prompt: "You are a helpful assistant.".to_string(),
            max_steps: 4,
        },
    );

    let answer = agent.run("session-1", "What is 7 + 5?").await?;
    println!("agent answer: {answer}");
    println!("event_count: {}", callbacks.events().await.len());
    Ok(())
}
