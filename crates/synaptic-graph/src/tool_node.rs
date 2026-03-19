use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::{Message, RuntimeAwareTool, Store, SynapticError, ToolRuntime};
use synaptic_events::{EmitResult, Event, EventBus, EventKind};
use synaptic_middleware::{InterceptorChain, ToolCallRequest, ToolCaller};
use synaptic_tools::SerialToolExecutor;

use crate::command::NodeOutput;
use crate::node::Node;
use crate::state::MessageState;

/// Wraps a `SerialToolExecutor` into a `ToolCaller` for the interceptor chain.
struct BaseToolCaller {
    executor: SerialToolExecutor,
}

#[async_trait]
impl ToolCaller for BaseToolCaller {
    async fn call(&self, request: ToolCallRequest) -> Result<Value, SynapticError> {
        self.executor
            .execute(&request.call.name, request.call.arguments.clone())
            .await
    }
}

/// Prebuilt node that executes tool calls from the last AI message in state.
///
/// Supports both regular `Tool` and `RuntimeAwareTool` instances.
/// When a runtime-aware tool is registered, it receives the current graph
/// state, store reference, and tool call ID via [`ToolRuntime`].
///
/// By default, tool calls are executed serially. Call [`ToolNode::with_parallel`]
/// to enable concurrent execution of multiple tool calls within a single step.
pub struct ToolNode {
    executor: SerialToolExecutor,
    interceptors: Option<Arc<InterceptorChain>>,
    event_bus: Option<Arc<EventBus>>,
    /// Optional store reference injected into RuntimeAwareTool calls.
    store: Option<Arc<dyn Store>>,
    /// Runtime-aware tools keyed by tool name.
    runtime_tools: HashMap<String, Arc<dyn RuntimeAwareTool>>,
    /// When true and multiple tool calls exist, execute them concurrently.
    parallel: bool,
}

impl ToolNode {
    pub fn new(executor: SerialToolExecutor) -> Self {
        Self {
            executor,
            interceptors: None,
            event_bus: None,
            store: None,
            runtime_tools: HashMap::new(),
            parallel: false,
        }
    }

    /// Create a ToolNode with interceptor chain support.
    pub fn with_interceptors(
        executor: SerialToolExecutor,
        interceptors: Arc<InterceptorChain>,
    ) -> Self {
        Self {
            executor,
            interceptors: Some(interceptors),
            event_bus: None,
            store: None,
            runtime_tools: HashMap::new(),
            parallel: false,
        }
    }

    /// Set the event bus for emitting tool lifecycle events.
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Enable parallel execution of tool calls.
    ///
    /// When enabled and multiple tool calls exist in the last AI message,
    /// they are executed concurrently using `futures::future::join_all`.
    /// Results are collected in the same order as the original tool calls.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set the store reference for runtime-aware tool injection.
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Register a runtime-aware tool.
    ///
    /// When a tool call matches a registered runtime-aware tool by name,
    /// it will be called with a [`ToolRuntime`] containing the current
    /// graph state, store, and tool call ID.
    pub fn with_runtime_tool(mut self, tool: Arc<dyn RuntimeAwareTool>) -> Self {
        self.runtime_tools.insert(tool.name().to_string(), tool);
        self
    }

    /// Emit an intercept-capable event. Returns the EmitResult.
    async fn emit_intercept_event(
        &self,
        kind: EventKind,
        payload: Value,
    ) -> Result<Option<EmitResult>, SynapticError> {
        if let Some(ref bus) = self.event_bus {
            let mut event = Event::new(kind, payload).with_source("graph_tools");
            let result = bus.emit(&mut event).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Emit a fire-and-forget event.
    async fn emit_event(&self, kind: EventKind, payload: Value) {
        if let Some(ref bus) = self.event_bus {
            let mut event = Event::new(kind, payload).with_source("graph_tools");
            let _ = bus.emit(&mut event).await;
        }
    }
}

#[async_trait]
impl Node<MessageState> for ToolNode {
    async fn process(
        &self,
        mut state: MessageState,
    ) -> Result<NodeOutput<MessageState>, SynapticError> {
        let last = state
            .last_message()
            .ok_or_else(|| SynapticError::Graph("no messages in state".to_string()))?;

        let tool_calls = last.tool_calls().to_vec();
        if tool_calls.is_empty() {
            return Ok(state.into());
        }

        // Serialize current state for context injection
        let state_value = serde_json::to_value(&state).ok();

        if self.parallel && tool_calls.len() > 1 {
            // Parallel execution: run all tool calls concurrently
            let futs: Vec<_> = tool_calls
                .iter()
                .map(|call| {
                    let executor = self.executor.clone();
                    let interceptors = self.interceptors.clone();
                    let rt_tool = self.runtime_tools.get(&call.name).cloned();
                    let store = self.store.clone();
                    let sv = state_value.clone();
                    let call = call.clone();
                    let event_bus = self.event_bus.clone();
                    async move {
                        // Emit BeforeToolCall
                        if let Some(ref bus) = event_bus {
                            let payload = serde_json::json!({
                                "tool_name": call.name,
                                "tool_call_id": call.id,
                                "arguments": call.arguments,
                            });
                            let mut event = Event::new(EventKind::BeforeToolCall, payload)
                                .with_source("graph_tools");
                            match bus.emit(&mut event).await {
                                Ok(EmitResult::Intercepted(val)) => return Ok(val),
                                Ok(EmitResult::Cancelled) => {
                                    return Ok(serde_json::json!({
                                        "error": "Tool call cancelled by event subscriber"
                                    }));
                                }
                                Err(e) => return Err(e),
                                _ => {}
                            }
                        }

                        let result = if let Some(rt) = rt_tool {
                            let runtime = ToolRuntime {
                                store,
                                stream_writer: None,
                                state: sv,
                                tool_call_id: call.id.clone(),
                                config: None,
                            };
                            rt.call_with_runtime(call.arguments.clone(), runtime).await
                        } else if let Some(ref chain) = interceptors {
                            let request = ToolCallRequest { call: call.clone() };
                            let base = BaseToolCaller { executor };
                            chain.call_tool(request, &base).await
                        } else {
                            executor.execute(&call.name, call.arguments.clone()).await
                        };

                        // Emit AfterToolCall
                        if let Some(ref bus) = event_bus {
                            let after_payload = serde_json::json!({
                                "tool_name": call.name,
                                "tool_call_id": call.id,
                                "success": result.is_ok(),
                            });
                            let mut after_event =
                                Event::new(EventKind::AfterToolCall, after_payload)
                                    .with_source("graph_tools");
                            let _ = bus.emit(&mut after_event).await;
                        }

                        result
                    }
                })
                .collect();
            let results = futures::future::join_all(futs).await;
            for (call, result) in tool_calls.iter().zip(results) {
                let content = match result {
                    Ok(val) => value_to_display_string(val),
                    Err(e) => format!("Error: {}", e),
                };
                state.messages.push(Message::tool(content, &call.id));
            }
        } else {
            // Serial execution (default)
            for call in &tool_calls {
                // Emit BeforeToolCall (Intercept mode)
                if let Some(result) = self
                    .emit_intercept_event(
                        EventKind::BeforeToolCall,
                        serde_json::json!({
                            "tool_name": call.name,
                            "tool_call_id": call.id,
                            "arguments": call.arguments,
                        }),
                    )
                    .await?
                {
                    match result {
                        EmitResult::Intercepted(val) => {
                            state
                                .messages
                                .push(Message::tool(value_to_display_string(val), &call.id));
                            continue;
                        }
                        EmitResult::Cancelled => {
                            state.messages.push(Message::tool(
                                "Tool call cancelled by event subscriber".to_string(),
                                &call.id,
                            ));
                            continue;
                        }
                        _ => {}
                    }
                }

                let result = if let Some(rt_tool) = self.runtime_tools.get(&call.name) {
                    let runtime = ToolRuntime {
                        store: self.store.clone(),
                        stream_writer: None,
                        state: state_value.clone(),
                        tool_call_id: call.id.clone(),
                        config: None,
                    };
                    rt_tool
                        .call_with_runtime(call.arguments.clone(), runtime)
                        .await
                } else if let Some(ref chain) = self.interceptors {
                    let request = ToolCallRequest { call: call.clone() };
                    let base = BaseToolCaller {
                        executor: self.executor.clone(),
                    };
                    chain.call_tool(request, &base).await
                } else {
                    self.executor
                        .execute(&call.name, call.arguments.clone())
                        .await
                };

                // Emit AfterToolCall (fire-and-forget)
                self.emit_event(
                    EventKind::AfterToolCall,
                    serde_json::json!({
                        "tool_name": call.name,
                        "tool_call_id": call.id,
                        "success": result.is_ok(),
                    }),
                )
                .await;

                let content = match result {
                    Ok(val) => value_to_display_string(val),
                    Err(e) => format!("Error: {}", e),
                };
                state.messages.push(Message::tool(content, &call.id));
            }
        }

        Ok(state.into())
    }
}

/// Convert a tool result Value to a display-friendly string.
/// For `Value::String`, returns the inner string (without JSON escaping).
/// For other types, uses `to_string()` (JSON serialization).
fn value_to_display_string(val: serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }
}

/// Standard routing function: returns "tools" if last message has tool_calls, else END.
///
/// This is the standard condition function used with `add_conditional_edges`
/// to route between an agent node and a tools node.
pub fn tools_condition(state: &MessageState) -> String {
    if let Some(last) = state.last_message() {
        if !last.tool_calls().is_empty() {
            return "tools".to_string();
        }
    }
    crate::END.to_string()
}
