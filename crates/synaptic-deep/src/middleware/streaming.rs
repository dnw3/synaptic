use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use synaptic_core::{ChatModel, Message, RunContext, SynapticError, TokenUsage, ToolCall};
use synaptic_graph::streaming::StreamingOutput;
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};

/// Newtype wrapper so `RunContext` can store `Arc<dyn StreamingOutput>` via
/// the opaque `Arc<dyn Any + Send + Sync>` slot.
///
/// Construct with [`StreamingOutputHandle::new`], attach to a `RunContext`
/// with [`RunContext::with_streaming_output`], and recover with
/// [`RunContext::streaming_output::<StreamingOutputHandle>()`].
#[derive(Clone)]
pub struct StreamingOutputHandle(pub Arc<dyn StreamingOutput>);

impl StreamingOutputHandle {
    pub fn new(output: Arc<dyn StreamingOutput>) -> Self {
        Self(output)
    }
}

/// Intercepts model calls to enable real-time token streaming.
///
/// When `RunContext` contains a [`StreamingOutputHandle`], calls
/// `stream_chat()` on the inner model and forwards token/reasoning deltas
/// to the [`StreamingOutput`]. When no handle is present, passes through
/// to the next caller unchanged.
pub struct StreamingInterceptor {
    model: Arc<dyn ChatModel>,
}

impl StreamingInterceptor {
    pub fn new(model: Arc<dyn ChatModel>) -> Self {
        Self { model }
    }
}

#[async_trait]
impl Interceptor for StreamingInterceptor {
    async fn wrap_model_call(
        &self,
        request: ModelRequest,
        ctx: &RunContext,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        // Check for a StreamingOutputHandle in the RunContext
        let handle: Arc<StreamingOutputHandle> =
            match ctx.streaming_output::<StreamingOutputHandle>() {
                Some(h) => h,
                None => return next.call(request, ctx).await,
            };
        let output = &handle.0;

        // Convert ModelRequest to ChatRequest and stream
        let chat_request = request.to_chat_request();
        let mut stream = self.model.stream_chat(chat_request);

        let mut content = String::new();
        let mut reasoning = String::new();
        let mut usage: Option<TokenUsage> = None;

        // Accumulate tool call chunks by index: (id, name, args_buffer)
        let mut tc_map: BTreeMap<usize, (String, String, String)> = BTreeMap::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;

            if !chunk.content.is_empty() {
                output.on_token(&chunk.content).await;
                content.push_str(&chunk.content);
            }
            if !chunk.reasoning.is_empty() {
                output.on_reasoning(&chunk.reasoning).await;
                reasoning.push_str(&chunk.reasoning);
            }

            // Merge tool_call_chunks by index
            for tc in &chunk.tool_call_chunks {
                let idx = tc.index.unwrap_or(0);
                let entry = tc_map
                    .entry(idx)
                    .or_insert_with(|| (String::new(), String::new(), String::new()));
                if let Some(ref id) = tc.id {
                    entry.0.clone_from(id);
                }
                if let Some(ref name) = tc.name {
                    entry.1.clone_from(name);
                }
                if let Some(ref args) = tc.arguments {
                    entry.2.push_str(args);
                }
            }

            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
        }

        // Build final tool calls from accumulated chunks
        let tool_calls: Vec<ToolCall> = tc_map
            .into_values()
            .filter(|(_, name, _)| !name.is_empty())
            .map(|(id, name, args_buf)| {
                let arguments = if args_buf.is_empty() {
                    serde_json::Value::Object(Default::default())
                } else {
                    serde_json::from_str(&args_buf)
                        .unwrap_or(serde_json::Value::Object(Default::default()))
                };
                ToolCall {
                    id,
                    name,
                    arguments,
                }
            })
            .collect();

        let message = if tool_calls.is_empty() {
            Message::ai(&content)
        } else {
            Message::ai_with_tool_calls(&content, tool_calls)
        };

        Ok(ModelResponse { message, usage })
    }
}
