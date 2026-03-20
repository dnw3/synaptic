use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::chat_model::TokenUsage;
use crate::tool::{InvalidToolCall, ToolCall, ToolCallChunk};

// ---------------------------------------------------------------------------
// ContentBlock — multimodal message content
// ---------------------------------------------------------------------------

/// A block of content within a message, supporting multimodal inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    Audio {
        url: String,
    },
    Video {
        url: String,
    },
    File {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    Data {
        data: Value,
    },
    Reasoning {
        content: String,
    },
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

/// Represents a chat message. Tagged enum with System, Human, AI, and Tool variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System {
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        response_metadata: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content_blocks: Vec<ContentBlock>,
    },
    #[serde(rename = "human")]
    Human {
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        response_metadata: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content_blocks: Vec<ContentBlock>,
    },
    #[serde(rename = "assistant")]
    AI {
        content: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        response_metadata: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content_blocks: Vec<ContentBlock>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage_metadata: Option<TokenUsage>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        invalid_tool_calls: Vec<InvalidToolCall>,
    },
    #[serde(rename = "tool")]
    Tool {
        content: String,
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        response_metadata: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content_blocks: Vec<ContentBlock>,
    },
    #[serde(rename = "chat")]
    Chat {
        custom_role: String,
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        response_metadata: HashMap<String, Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content_blocks: Vec<ContentBlock>,
    },
    /// A special message that signals removal of a message by its ID.
    /// Used in message history management.
    #[serde(rename = "remove")]
    Remove {
        /// ID of the message to remove.
        id: String,
    },
}

/// Helper macro to set a shared field across all Message variants.
/// Note: Remove variant has no common fields, so it is a no-op.
macro_rules! set_message_field {
    ($self:expr, $field:ident, $value:expr) => {
        match $self {
            Message::System { $field, .. } => *$field = $value,
            Message::Human { $field, .. } => *$field = $value,
            Message::AI { $field, .. } => *$field = $value,
            Message::Tool { $field, .. } => *$field = $value,
            Message::Chat { $field, .. } => *$field = $value,
            Message::Remove { .. } => { /* Remove has no common fields */ }
        }
    };
}

/// Helper macro to get a shared field from all Message variants.
/// Note: Remove variant panics — callers handle Remove before using this macro.
macro_rules! get_message_field {
    ($self:expr, $field:ident) => {
        match $self {
            Message::System { $field, .. } => $field,
            Message::Human { $field, .. } => $field,
            Message::AI { $field, .. } => $field,
            Message::Tool { $field, .. } => $field,
            Message::Chat { $field, .. } => $field,
            Message::Remove { .. } => unreachable!("get_message_field called on Remove variant"),
        }
    };
}

impl Message {
    // -- Factory methods -----------------------------------------------------

    pub fn system(content: impl Into<String>) -> Self {
        Message::System {
            content: content.into(),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
            content_blocks: Vec::new(),
        }
    }

    pub fn human(content: impl Into<String>) -> Self {
        Message::Human {
            content: content.into(),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
            content_blocks: Vec::new(),
        }
    }

    pub fn ai(content: impl Into<String>) -> Self {
        Message::AI {
            content: content.into(),
            tool_calls: vec![],
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
            content_blocks: Vec::new(),
            usage_metadata: None,
            invalid_tool_calls: Vec::new(),
        }
    }

    pub fn ai_with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Message::AI {
            content: content.into(),
            tool_calls,
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
            content_blocks: Vec::new(),
            usage_metadata: None,
            invalid_tool_calls: Vec::new(),
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Message::Tool {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
            content_blocks: Vec::new(),
        }
    }

    pub fn chat(role: impl Into<String>, content: impl Into<String>) -> Self {
        Message::Chat {
            custom_role: role.into(),
            content: content.into(),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
            content_blocks: Vec::new(),
        }
    }

    /// Create a Remove message that signals removal of a message by its ID.
    pub fn remove(id: impl Into<String>) -> Self {
        Message::Remove { id: id.into() }
    }

    // -- Builder methods -----------------------------------------------------

    pub fn with_id(mut self, value: impl Into<String>) -> Self {
        set_message_field!(&mut self, id, Some(value.into()));
        self
    }

    pub fn with_name(mut self, value: impl Into<String>) -> Self {
        set_message_field!(&mut self, name, Some(value.into()));
        self
    }

    pub fn with_additional_kwarg(mut self, key: impl Into<String>, value: Value) -> Self {
        match &mut self {
            Message::System {
                additional_kwargs, ..
            }
            | Message::Human {
                additional_kwargs, ..
            }
            | Message::AI {
                additional_kwargs, ..
            }
            | Message::Tool {
                additional_kwargs, ..
            }
            | Message::Chat {
                additional_kwargs, ..
            } => {
                additional_kwargs.insert(key.into(), value);
            }
            Message::Remove { .. } => { /* Remove has no additional_kwargs */ }
        }
        self
    }

    pub fn with_response_metadata_entry(mut self, key: impl Into<String>, value: Value) -> Self {
        match &mut self {
            Message::System {
                response_metadata, ..
            }
            | Message::Human {
                response_metadata, ..
            }
            | Message::AI {
                response_metadata, ..
            }
            | Message::Tool {
                response_metadata, ..
            }
            | Message::Chat {
                response_metadata, ..
            } => {
                response_metadata.insert(key.into(), value);
            }
            Message::Remove { .. } => { /* Remove has no response_metadata */ }
        }
        self
    }

    pub fn with_content_blocks(mut self, blocks: Vec<ContentBlock>) -> Self {
        set_message_field!(&mut self, content_blocks, blocks);
        self
    }

    pub fn with_usage_metadata(mut self, usage: TokenUsage) -> Self {
        if let Message::AI { usage_metadata, .. } = &mut self {
            *usage_metadata = Some(usage);
        }
        self
    }

    // -- Accessor methods ----------------------------------------------------

    pub fn content(&self) -> &str {
        match self {
            Message::Remove { .. } => "",
            other => get_message_field!(other, content),
        }
    }

    pub fn role(&self) -> &str {
        match self {
            Message::System { .. } => "system",
            Message::Human { .. } => "human",
            Message::AI { .. } => "assistant",
            Message::Tool { .. } => "tool",
            Message::Chat { custom_role, .. } => custom_role,
            Message::Remove { .. } => "remove",
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

    pub fn is_chat(&self) -> bool {
        matches!(self, Message::Chat { .. })
    }

    pub fn is_remove(&self) -> bool {
        matches!(self, Message::Remove { .. })
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

    pub fn id(&self) -> Option<&str> {
        match self {
            Message::Remove { id } => Some(id),
            other => get_message_field!(other, id).as_deref(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Message::Remove { .. } => None,
            other => get_message_field!(other, name).as_deref(),
        }
    }

    pub fn additional_kwargs(&self) -> &HashMap<String, Value> {
        match self {
            Message::System {
                additional_kwargs, ..
            }
            | Message::Human {
                additional_kwargs, ..
            }
            | Message::AI {
                additional_kwargs, ..
            }
            | Message::Tool {
                additional_kwargs, ..
            }
            | Message::Chat {
                additional_kwargs, ..
            } => additional_kwargs,
            Message::Remove { .. } => {
                static EMPTY: std::sync::OnceLock<HashMap<String, Value>> =
                    std::sync::OnceLock::new();
                EMPTY.get_or_init(HashMap::new)
            }
        }
    }

    pub fn response_metadata(&self) -> &HashMap<String, Value> {
        match self {
            Message::System {
                response_metadata, ..
            }
            | Message::Human {
                response_metadata, ..
            }
            | Message::AI {
                response_metadata, ..
            }
            | Message::Tool {
                response_metadata, ..
            }
            | Message::Chat {
                response_metadata, ..
            } => response_metadata,
            Message::Remove { .. } => {
                static EMPTY: std::sync::OnceLock<HashMap<String, Value>> =
                    std::sync::OnceLock::new();
                EMPTY.get_or_init(HashMap::new)
            }
        }
    }

    pub fn content_blocks(&self) -> &[ContentBlock] {
        match self {
            Message::Remove { .. } => &[],
            other => get_message_field!(other, content_blocks),
        }
    }

    /// Return the remove ID if this is a Remove message.
    pub fn remove_id(&self) -> Option<&str> {
        match self {
            Message::Remove { id } => Some(id),
            _ => None,
        }
    }

    pub fn usage_metadata(&self) -> Option<&TokenUsage> {
        match self {
            Message::AI { usage_metadata, .. } => usage_metadata.as_ref(),
            _ => None,
        }
    }

    pub fn invalid_tool_calls(&self) -> &[InvalidToolCall] {
        match self {
            Message::AI {
                invalid_tool_calls, ..
            } => invalid_tool_calls,
            _ => &[],
        }
    }

    /// Set the content of this message. No-op for Remove variant.
    pub fn set_content(&mut self, new_content: impl Into<String>) {
        let new_content = new_content.into();
        set_message_field!(self, content, new_content);
    }
}

// ---------------------------------------------------------------------------
// Message utility functions
// ---------------------------------------------------------------------------

/// Filter messages by type, name, or id.
pub fn filter_messages(
    messages: &[Message],
    include_types: Option<&[&str]>,
    exclude_types: Option<&[&str]>,
    include_names: Option<&[&str]>,
    exclude_names: Option<&[&str]>,
    include_ids: Option<&[&str]>,
    exclude_ids: Option<&[&str]>,
) -> Vec<Message> {
    messages
        .iter()
        .filter(|msg| {
            if let Some(include) = include_types {
                if !include.contains(&msg.role()) {
                    return false;
                }
            }
            if let Some(exclude) = exclude_types {
                if exclude.contains(&msg.role()) {
                    return false;
                }
            }
            if let Some(include) = include_names {
                match msg.name() {
                    Some(name) => {
                        if !include.contains(&name) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            if let Some(exclude) = exclude_names {
                if let Some(name) = msg.name() {
                    if exclude.contains(&name) {
                        return false;
                    }
                }
            }
            if let Some(include) = include_ids {
                match msg.id() {
                    Some(id) => {
                        if !include.contains(&id) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            if let Some(exclude) = exclude_ids {
                if let Some(id) = msg.id() {
                    if exclude.contains(&id) {
                        return false;
                    }
                }
            }
            true
        })
        .cloned()
        .collect()
}

/// Strategy for trimming messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStrategy {
    /// Keep the first messages that fit within the token budget.
    First,
    /// Keep the last messages that fit within the token budget.
    Last,
}

/// Trim messages to fit within a token budget.
///
/// `token_counter` receives a single message and returns its token count.
/// When `include_system` is true and `strategy` is `Last`, the leading system
/// message is always preserved.
pub fn trim_messages(
    messages: Vec<Message>,
    max_tokens: usize,
    token_counter: impl Fn(&Message) -> usize,
    strategy: TrimStrategy,
    include_system: bool,
) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    match strategy {
        TrimStrategy::First => {
            let mut result = Vec::new();
            let mut total = 0;
            for msg in messages {
                let count = token_counter(&msg);
                if total + count > max_tokens {
                    break;
                }
                total += count;
                result.push(msg);
            }
            result
        }
        TrimStrategy::Last => {
            let (system_msg, rest) = if include_system && messages[0].is_system() {
                (Some(messages[0].clone()), &messages[1..])
            } else {
                (None, messages.as_slice())
            };

            let system_tokens = system_msg.as_ref().map(&token_counter).unwrap_or(0);
            let budget = max_tokens.saturating_sub(system_tokens);

            let mut selected = Vec::new();
            let mut total = 0;
            for msg in rest.iter().rev() {
                let count = token_counter(msg);
                if total + count > budget {
                    break;
                }
                total += count;
                selected.push(msg.clone());
            }
            selected.reverse();

            let mut result = Vec::new();
            if let Some(sys) = system_msg {
                result.push(sys);
            }
            result.extend(selected);
            result
        }
    }
}

/// Merge consecutive messages of the same role into a single message.
pub fn merge_message_runs(messages: Vec<Message>) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    let mut result: Vec<Message> = Vec::new();

    for msg in messages {
        let should_merge = result
            .last()
            .map(|last| last.role() == msg.role())
            .unwrap_or(false);

        if should_merge {
            let last = result.last_mut().unwrap();
            // Merge content
            let merged_content = format!("{}\n{}", last.content(), msg.content());
            match last {
                Message::System { content, .. } => *content = merged_content,
                Message::Human { content, .. } => *content = merged_content,
                Message::AI {
                    content,
                    tool_calls,
                    invalid_tool_calls,
                    ..
                } => {
                    *content = merged_content;
                    tool_calls.extend(msg.tool_calls().to_vec());
                    invalid_tool_calls.extend(msg.invalid_tool_calls().to_vec());
                }
                Message::Tool { content, .. } => *content = merged_content,
                Message::Chat { content, .. } => *content = merged_content,
                Message::Remove { .. } => { /* Remove messages are not merged */ }
            }
        } else {
            result.push(msg);
        }
    }

    result
}

/// Convert messages to a human-readable buffer string.
pub fn get_buffer_string(messages: &[Message], human_prefix: &str, ai_prefix: &str) -> String {
    messages
        .iter()
        .map(|msg| {
            let prefix = match msg {
                Message::System { .. } => "System",
                Message::Human { .. } => human_prefix,
                Message::AI { .. } => ai_prefix,
                Message::Tool { .. } => "Tool",
                Message::Chat { custom_role, .. } => custom_role.as_str(),
                Message::Remove { .. } => "Remove",
            };
            format!("{prefix}: {}", msg.content())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// AIMessageChunk
// ---------------------------------------------------------------------------

/// A streaming chunk from an AI model response. Supports merge via `+`/`+=` operators and conversion to `Message` via `into_message()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AIMessageChunk {
    pub content: String,
    /// Reasoning / thinking content from extended thinking models.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reasoning: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_call_chunks: Vec<ToolCallChunk>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_tool_calls: Vec<InvalidToolCall>,
}

impl AIMessageChunk {
    pub fn into_message(self) -> Message {
        let mut msg = Message::ai_with_tool_calls(self.content, self.tool_calls);
        if !self.reasoning.is_empty() {
            msg = msg.with_content_blocks(vec![ContentBlock::Reasoning {
                content: self.reasoning,
            }]);
        }
        msg
    }
}

impl std::ops::Add for AIMessageChunk {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl std::ops::AddAssign for AIMessageChunk {
    fn add_assign(&mut self, rhs: Self) {
        self.content.push_str(&rhs.content);
        self.reasoning.push_str(&rhs.reasoning);
        self.tool_calls.extend(rhs.tool_calls);
        self.tool_call_chunks.extend(rhs.tool_call_chunks);
        self.invalid_tool_calls.extend(rhs.invalid_tool_calls);
        if self.id.is_none() {
            self.id = rhs.id;
        }
        match (&mut self.usage, rhs.usage) {
            (Some(u), Some(rhs_u)) => {
                u.input_tokens += rhs_u.input_tokens;
                u.output_tokens += rhs_u.output_tokens;
                u.total_tokens += rhs_u.total_tokens;
            }
            (None, Some(rhs_u)) => {
                self.usage = Some(rhs_u);
            }
            _ => {}
        }
    }
}
