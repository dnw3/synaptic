use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System { content: String },
    #[serde(rename = "human")]
    Human { content: String },
    #[serde(rename = "assistant")]
    AI {
        content: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    #[serde(rename = "tool")]
    Tool {
        content: String,
        tool_call_id: String,
    },
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Message::System {
            content: content.into(),
        }
    }

    pub fn human(content: impl Into<String>) -> Self {
        Message::Human {
            content: content.into(),
        }
    }

    pub fn ai(content: impl Into<String>) -> Self {
        Message::AI {
            content: content.into(),
            tool_calls: vec![],
        }
    }

    pub fn ai_with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Message::AI {
            content: content.into(),
            tool_calls,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Message::Tool {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Message::System { content } => content,
            Message::Human { content } => content,
            Message::AI { content, .. } => content,
            Message::Tool { content, .. } => content,
        }
    }

    pub fn role(&self) -> &str {
        match self {
            Message::System { .. } => "system",
            Message::Human { .. } => "human",
            Message::AI { .. } => "assistant",
            Message::Tool { .. } => "tool",
        }
    }

    pub fn is_system(&self) -> bool {
        matches!(self, Message::System { .. })
    }

    pub fn is_human(&self) -> bool {
        matches!(self, Message::Human { .. })
    }

    pub fn is_ai(&self) -> bool {
        matches!(self, Message::AI { .. })
    }

    pub fn is_tool(&self) -> bool {
        matches!(self, Message::Tool { .. })
    }

    pub fn tool_calls(&self) -> &[ToolCall] {
        match self {
            Message::AI { tool_calls, .. } => tool_calls,
            _ => &[],
        }
    }

    pub fn tool_call_id(&self) -> Option<&str> {
        match self {
            Message::Tool { tool_call_id, .. } => Some(tool_call_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
