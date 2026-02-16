use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunEvent {
    RunStarted {
        run_id: String,
        session_id: String,
    },
    RunStep {
        run_id: String,
        step: usize,
    },
    LlmCalled {
        run_id: String,
        message_count: usize,
    },
    ToolCalled {
        run_id: String,
        tool_name: String,
    },
    RunFinished {
        run_id: String,
        output: String,
    },
    RunFailed {
        run_id: String,
        error: String,
    },
}

#[derive(Debug, Error)]
pub enum SynapseError {
    #[error("prompt error: {0}")]
    Prompt(String),
    #[error("model error: {0}")]
    Model(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("memory error: {0}")]
    Memory(String),
    #[error("rate limit: {0}")]
    RateLimit(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("parsing error: {0}")]
    Parsing(String),
    #[error("callback error: {0}")]
    Callback(String),
    #[error("max steps exceeded: {max_steps}")]
    MaxStepsExceeded { max_steps: usize },
}

#[async_trait]
pub trait ChatModel: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError>;
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn call(&self, args: Value) -> Result<Value, SynapseError>;
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn append(&self, session_id: &str, message: Message) -> Result<(), SynapseError>;
    async fn load(&self, session_id: &str) -> Result<Vec<Message>, SynapseError>;
    async fn clear(&self, session_id: &str) -> Result<(), SynapseError>;
}

#[async_trait]
pub trait CallbackHandler: Send + Sync {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapseError>;
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(&self, session_id: &str, input: &str) -> Result<String, SynapseError>;
}
