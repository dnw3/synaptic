use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use synapse_core::{
    AIMessageChunk, ChatModel, ChatRequest, ChatResponse, ChatStream, Message, SynapseError,
    TokenUsage, ToolCall, ToolChoice, ToolDefinition,
};

use crate::backend::{ProviderBackend, ProviderRequest, ProviderResponse};

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_tokens: None,
            temperature: None,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

pub struct OpenAiChatModel {
    config: OpenAiConfig,
    backend: Arc<dyn ProviderBackend>,
}

impl OpenAiChatModel {
    pub fn new(config: OpenAiConfig, backend: Arc<dyn ProviderBackend>) -> Self {
        Self { config, backend }
    }

    fn build_request(&self, request: &ChatRequest, stream: bool) -> ProviderRequest {
        let messages: Vec<Value> = request.messages.iter().map(message_to_openai).collect();

        let mut body = json!({
            "model": self.config.model,
            "messages": messages,
            "stream": stream,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = self.config.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(request
                .tools
                .iter()
                .map(tool_def_to_openai)
                .collect::<Vec<_>>());
        }
        if let Some(ref choice) = request.tool_choice {
            body["tool_choice"] = match choice {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::Required => json!("required"),
                ToolChoice::None => json!("none"),
                ToolChoice::Specific(name) => json!({
                    "type": "function",
                    "function": {"name": name}
                }),
            };
        }

        ProviderRequest {
            url: format!("{}/chat/completions", self.config.base_url),
            headers: vec![
                (
                    "Authorization".to_string(),
                    format!("Bearer {}", self.config.api_key),
                ),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body,
        }
    }
}

fn message_to_openai(msg: &Message) -> Value {
    match msg {
        Message::System { content } => json!({
            "role": "system",
            "content": content,
        }),
        Message::Human { content } => json!({
            "role": "user",
            "content": content,
        }),
        Message::AI {
            content,
            tool_calls,
        } => {
            let mut obj = json!({
                "role": "assistant",
                "content": content,
            });
            if !tool_calls.is_empty() {
                obj["tool_calls"] = json!(tool_calls
                    .iter()
                    .map(|tc| json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string(),
                        }
                    }))
                    .collect::<Vec<_>>());
            }
            obj
        }
        Message::Tool {
            content,
            tool_call_id,
        } => json!({
            "role": "tool",
            "content": content,
            "tool_call_id": tool_call_id,
        }),
    }
}

fn tool_def_to_openai(def: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": def.name,
            "description": def.description,
            "parameters": def.parameters,
        }
    })
}

fn parse_response(resp: &ProviderResponse) -> Result<ChatResponse, SynapseError> {
    check_error_status(resp)?;

    let choice = &resp.body["choices"][0]["message"];
    let content = choice["content"].as_str().unwrap_or("").to_string();
    let tool_calls = parse_tool_calls(choice);

    let usage = parse_usage(&resp.body["usage"]);

    let message = if tool_calls.is_empty() {
        Message::ai(content)
    } else {
        Message::ai_with_tool_calls(content, tool_calls)
    };

    Ok(ChatResponse { message, usage })
}

fn check_error_status(resp: &ProviderResponse) -> Result<(), SynapseError> {
    if resp.status == 429 {
        let msg = resp.body["error"]["message"]
            .as_str()
            .unwrap_or("rate limited")
            .to_string();
        return Err(SynapseError::RateLimit(msg));
    }
    if resp.status >= 400 {
        let msg = resp.body["error"]["message"]
            .as_str()
            .unwrap_or("unknown API error")
            .to_string();
        return Err(SynapseError::Model(format!(
            "OpenAI API error ({}): {}",
            resp.status, msg
        )));
    }
    Ok(())
}

fn parse_tool_calls(message: &Value) -> Vec<ToolCall> {
    message["tool_calls"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    let id = tc["id"].as_str()?.to_string();
                    let name = tc["function"]["name"].as_str()?.to_string();
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let arguments =
                        serde_json::from_str(args_str).unwrap_or(Value::Object(Default::default()));
                    Some(ToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_usage(usage: &Value) -> Option<TokenUsage> {
    if usage.is_null() {
        return None;
    }
    Some(TokenUsage {
        input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
        total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
    })
}

fn parse_stream_chunk(data: &str) -> Option<AIMessageChunk> {
    let v: Value = serde_json::from_str(data).ok()?;
    let delta = &v["choices"][0]["delta"];

    let content = delta["content"].as_str().unwrap_or("").to_string();
    let tool_calls = parse_tool_calls(delta);
    let usage = parse_usage(&v["usage"]);

    Some(AIMessageChunk {
        content,
        tool_calls,
        usage,
    })
}

#[async_trait]
impl ChatModel for OpenAiChatModel {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError> {
        let provider_req = self.build_request(&request, false);
        let resp = self.backend.send(provider_req).await?;
        parse_response(&resp)
    }

    fn stream_chat(&self, request: ChatRequest) -> ChatStream<'_> {
        Box::pin(async_stream::stream! {
            let provider_req = self.build_request(&request, true);
            let byte_stream = self.backend.send_stream(provider_req).await;

            let byte_stream = match byte_stream {
                Ok(s) => s,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            use eventsource_stream::Eventsource;
            use futures::StreamExt;

            let mut event_stream = byte_stream
                .map(|result| result.map_err(|e| std::io::Error::other(e.to_string())))
                .eventsource();

            while let Some(event) = event_stream.next().await {
                match event {
                    Ok(ev) => {
                        if ev.data == "[DONE]" {
                            break;
                        }
                        if let Some(chunk) = parse_stream_chunk(&ev.data) {
                            yield Ok(chunk);
                        }
                    }
                    Err(e) => {
                        yield Err(SynapseError::Model(format!("SSE parse error: {e}")));
                        break;
                    }
                }
            }
        })
    }
}
