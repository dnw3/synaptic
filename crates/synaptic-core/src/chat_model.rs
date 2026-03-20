use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::error::SynapticError;
use crate::message::{AIMessageChunk, Message};
use crate::tool::ToolDefinition;

// ---------------------------------------------------------------------------
// Token usage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_details: Option<InputTokenDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_details: Option<OutputTokenDetails>,
}

/// Detailed breakdown of input token usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InputTokenDetails {
    #[serde(default)]
    pub cached: u32,
    #[serde(default)]
    pub audio: u32,
}

/// Detailed breakdown of output token usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutputTokenDetails {
    #[serde(default)]
    pub reasoning: u32,
    #[serde(default)]
    pub audio: u32,
}

// ---------------------------------------------------------------------------
// ThinkingConfig
// ---------------------------------------------------------------------------

/// Configuration for extended thinking / reasoning mode.
///
/// When enabled, models that support it (e.g. Anthropic Claude) will produce
/// detailed reasoning before their final response. The `budget_tokens` field
/// controls how many tokens the model may use for reasoning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Whether extended thinking is enabled.
    pub enabled: bool,
    /// Maximum number of tokens the model may use for reasoning.
    /// If None, the provider's default budget is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

// ---------------------------------------------------------------------------
// Chat request / response
// ---------------------------------------------------------------------------

/// A request to a chat model containing messages, optional tool definitions, and tool choice configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<crate::tool::ToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

impl ChatRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            tools: vec![],
            tool_choice: None,
            thinking: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_tool_choice(mut self, choice: crate::tool::ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    pub fn with_thinking(mut self, config: ThinkingConfig) -> Self {
        self.thinking = Some(config);
        self
    }
}

/// A response from a chat model containing the AI message and optional token usage statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: Message,
    pub usage: Option<TokenUsage>,
}

// ---------------------------------------------------------------------------
// ModelProfile
// ---------------------------------------------------------------------------

/// Describes a model's capabilities and limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub name: String,
    pub provider: String,
    pub supports_tool_calling: bool,
    pub supports_structured_output: bool,
    pub supports_streaming: bool,
    pub max_input_tokens: Option<usize>,
    pub max_output_tokens: Option<usize>,
}

// ---------------------------------------------------------------------------
// ChatStream
// ---------------------------------------------------------------------------

/// Type alias for a pinned, boxed async stream of `AIMessageChunk` results.
pub type ChatStream<'a> =
    Pin<Box<dyn Stream<Item = Result<AIMessageChunk, SynapticError>> + Send + 'a>>;

// ---------------------------------------------------------------------------
// ChatModel trait
// ---------------------------------------------------------------------------

/// The core trait for language model providers. Implementations provide `chat()` for single responses and optionally `stream_chat()` for streaming.
#[async_trait]
pub trait ChatModel: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapticError>;

    /// Return the model's capability profile, if known.
    fn profile(&self) -> Option<ModelProfile> {
        None
    }

    fn stream_chat(&self, request: ChatRequest) -> ChatStream<'_> {
        Box::pin(async_stream::stream! {
            match self.chat(request).await {
                Ok(response) => {
                    yield Ok(AIMessageChunk {
                        content: response.message.content().to_string(),
                        tool_calls: response.message.tool_calls().to_vec(),
                        usage: response.usage,
                        ..Default::default()
                    });
                }
                Err(e) => yield Err(e),
            }
        })
    }
}
