use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::{
    ChatModel, ChatRequest, Message, RunContext, SynapticError, Tool, ToolDefinition,
};
use synaptic_events::{EmitResult, Event, EventBus, EventKind};
use synaptic_macros::traceable;
use synaptic_middleware::{BaseChatModelCaller, Interceptor, InterceptorChain, ModelRequest};
use synaptic_store::Store;
use synaptic_tools::SerialToolExecutor;

use crate::builder::StateGraph;
use crate::checkpoint::Checkpointer;
use crate::command::NodeOutput;
use crate::compiled::CompiledGraph;
use crate::node::Node;
use crate::state::MessageState;
use crate::tool_node::ToolNode;
use crate::END;

// ---------------------------------------------------------------------------
// Hook types
// ---------------------------------------------------------------------------

/// A hook called before each model invocation. Can modify the state.
pub type PreModelHook = Arc<
    dyn Fn(
            &mut MessageState,
        ) -> Pin<Box<dyn Future<Output = Result<(), SynapticError>> + Send + '_>>
        + Send
        + Sync,
>;

/// A hook called after each model invocation. Can modify the state.
pub type PostModelHook = Arc<
    dyn Fn(
            &mut MessageState,
        ) -> Pin<Box<dyn Future<Output = Result<(), SynapticError>> + Send + '_>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// ChatModelNode — prebuilt node that calls a ChatModel through interceptors
// ---------------------------------------------------------------------------

struct ChatModelNode {
    model: Arc<dyn ChatModel>,
    tool_defs: Vec<ToolDefinition>,
    system_prompt: Option<String>,
    interceptors: Arc<InterceptorChain>,
    event_bus: Option<Arc<EventBus>>,
    is_first_call: AtomicBool,
    pre_model_hook: Option<PreModelHook>,
    post_model_hook: Option<PostModelHook>,
    /// When set, the final response (no tool calls) is re-called with
    /// structured output instructions matching this JSON schema.
    response_format: Option<Value>,
    /// Shared run context set by `invoke_with_context` / `stream_with_context`.
    run_context: Arc<tokio::sync::RwLock<RunContext>>,
}

impl ChatModelNode {
    /// Emit an event on the event bus if present. Errors are logged but not propagated.
    async fn emit_event(&self, kind: EventKind, payload: Value) {
        if let Some(ref bus) = self.event_bus {
            let mut event = Event::new(kind, payload).with_source("graph_agent");
            let _ = bus.emit(&mut event).await;
        }
    }

    /// Emit an intercept-capable event. Returns the EmitResult.
    async fn emit_intercept_event(
        &self,
        kind: EventKind,
        payload: Value,
    ) -> Result<Option<EmitResult>, SynapticError> {
        if let Some(ref bus) = self.event_bus {
            let mut event = Event::new(kind, payload).with_source("graph_agent");
            let result = bus.emit(&mut event).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl Node<MessageState> for ChatModelNode {
    async fn process(
        &self,
        mut state: MessageState,
    ) -> Result<NodeOutput<MessageState>, SynapticError> {
        // On first call, emit AgentStart
        if self.is_first_call.swap(false, Ordering::SeqCst) {
            self.emit_event(
                EventKind::AgentStart,
                serde_json::json!({
                    "message_count": state.messages.len(),
                }),
            )
            .await;
        }

        // Run pre_model_hook
        if let Some(ref hook) = self.pre_model_hook {
            hook(&mut state).await?;
        }

        let request = ModelRequest {
            messages: state.messages.clone(),
            tools: self.tool_defs.clone(),
            tool_choice: None,
            system_prompt: self.system_prompt.clone(),
            thinking: None,
        };

        // Emit BeforeModelCall (Intercept mode)
        if let Some(result) = self
            .emit_intercept_event(
                EventKind::BeforeModelCall,
                serde_json::json!({
                    "message_count": request.messages.len(),
                    "tool_count": request.tools.len(),
                }),
            )
            .await?
        {
            match result {
                EmitResult::Intercepted(val) => {
                    // Subscriber provided a synthetic response
                    let text = val
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| val.to_string());
                    let message = Message::ai(&text);
                    state.messages.push(message);
                    return Ok(state.into());
                }
                EmitResult::Cancelled => {
                    return Err(SynapticError::Tool(
                        "BeforeModelCall cancelled by event subscriber".to_string(),
                    ));
                }
                _ => {}
            }
        }

        // Call model through interceptor chain
        let base_caller = BaseChatModelCaller::new(self.model.clone());
        let ctx = self.run_context.read().await.clone();
        let response = match self
            .interceptors
            .call_model(request, &ctx, &base_caller)
            .await
        {
            Ok(resp) => {
                // Emit LlmOutput (fire-and-forget)
                let content_preview: String = resp.message.content().chars().take(500).collect();
                let tool_call_count = resp.message.tool_calls().len();
                let mut output_payload = serde_json::json!({
                    "content_preview": content_preview,
                    "tool_call_count": tool_call_count,
                });
                if let Some(ref u) = resp.usage {
                    output_payload["input_tokens"] = serde_json::json!(u.input_tokens);
                    output_payload["output_tokens"] = serde_json::json!(u.output_tokens);
                    output_payload["total_tokens"] = serde_json::json!(u.total_tokens);
                }
                self.emit_event(EventKind::LlmOutput, output_payload).await;
                resp
            }
            Err(e) => {
                // Emit OnModelError (fire-and-forget)
                self.emit_event(
                    EventKind::OnModelError,
                    serde_json::json!({
                        "error": e.to_string(),
                    }),
                )
                .await;
                return Err(e);
            }
        };

        state.messages.push(response.message.clone());

        // Run post_model_hook
        if let Some(ref hook) = self.post_model_hook {
            hook(&mut state).await?;
        }

        // If no tool calls, this is the final answer
        if response.message.tool_calls().is_empty() {
            // If response_format is set, re-call with structured output instructions
            if let Some(ref schema) = self.response_format {
                let instruction = format!(
                    "You MUST respond with valid JSON matching this schema:\n{}\n\n\
                     Do not include any text outside the JSON object. \
                     Do not use markdown code blocks.",
                    schema
                );
                let mut structured_messages = vec![Message::system(instruction)];
                structured_messages.extend(state.messages.clone());

                let structured_request = ChatRequest::new(structured_messages);
                let structured_response = self.model.chat(structured_request).await?;
                // Replace the last message with the structured response
                state.messages.pop();
                state.messages.push(structured_response.message);
            }

            // Emit AgentEnd with messages for ReflectionMiddleware
            self.emit_event(
                EventKind::AgentEnd,
                serde_json::json!({
                    "messages": state.messages,
                    "message_count": state.messages.len(),
                }),
            )
            .await;
        }

        Ok(state.into())
    }
}

// ---------------------------------------------------------------------------
// ReactAgentOptions (legacy)
// ---------------------------------------------------------------------------

/// Options for creating a ReAct agent with `create_react_agent_with_options`.
#[derive(Default)]
pub struct ReactAgentOptions {
    /// Optional checkpointer for state persistence across invocations.
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    /// Node names that should interrupt BEFORE execution (human-in-the-loop).
    pub interrupt_before: Vec<String>,
    /// Node names that should interrupt AFTER execution (human-in-the-loop).
    pub interrupt_after: Vec<String>,
    /// Optional system prompt to prepend to messages before calling the model.
    pub system_prompt: Option<String>,
}

/// Create a prebuilt ReAct agent graph.
pub fn create_react_agent(
    model: Arc<dyn ChatModel>,
    tools: Vec<Arc<dyn Tool>>,
) -> Result<CompiledGraph<MessageState>, SynapticError> {
    create_react_agent_with_options(model, tools, ReactAgentOptions::default())
}

/// Create a prebuilt ReAct agent graph with additional configuration options.
pub fn create_react_agent_with_options(
    model: Arc<dyn ChatModel>,
    tools: Vec<Arc<dyn Tool>>,
    options: ReactAgentOptions,
) -> Result<CompiledGraph<MessageState>, SynapticError> {
    create_agent(
        model,
        tools,
        AgentOptions {
            checkpointer: options.checkpointer,
            interrupt_before: options.interrupt_before,
            interrupt_after: options.interrupt_after,
            system_prompt: options.system_prompt,
            ..Default::default()
        },
    )
}

// ---------------------------------------------------------------------------
// AgentOptions — unified options for create_agent
// ---------------------------------------------------------------------------

/// Options for creating an agent with `create_agent`.
#[derive(Default)]
pub struct AgentOptions {
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub interrupt_before: Vec<String>,
    pub interrupt_after: Vec<String>,
    pub system_prompt: Option<String>,
    /// Interceptors forming the model/tool call chain.
    pub interceptors: Vec<Arc<dyn Interceptor>>,
    pub store: Option<Arc<dyn Store>>,
    pub name: Option<String>,
    pub pre_model_hook: Option<PreModelHook>,
    pub post_model_hook: Option<PostModelHook>,
    /// Optional JSON schema for structured output on the final model call.
    pub response_format: Option<Value>,
    /// Enable parallel tool execution in ToolNode (default false).
    pub parallel_tools: bool,
    /// Maximum graph iterations before aborting (default 100).
    /// Maps to `CompiledGraph.max_iterations`.
    pub max_iterations: Option<usize>,
    /// Optional event bus for emitting lifecycle events from graph nodes.
    pub event_bus: Option<Arc<EventBus>>,
}

/// Create a prebuilt agent graph with interceptor chain and event bus support.
#[traceable(skip = "model,tools,options")]
pub fn create_agent(
    model: Arc<dyn ChatModel>,
    tools: Vec<Arc<dyn Tool>>,
    options: AgentOptions,
) -> Result<CompiledGraph<MessageState>, SynapticError> {
    let tool_defs: Vec<ToolDefinition> = tools.iter().map(|t| t.as_tool_definition()).collect();

    let registry = synaptic_tools::ToolRegistry::new();
    for tool in tools {
        registry.register(tool)?;
    }
    let executor = SerialToolExecutor::new(registry);

    let interceptor_chain = Arc::new(InterceptorChain::new(options.interceptors));
    let shared_run_context = Arc::new(tokio::sync::RwLock::new(RunContext::default()));

    let agent_node = ChatModelNode {
        model,
        tool_defs,
        system_prompt: options.system_prompt,
        interceptors: interceptor_chain.clone(),
        event_bus: options.event_bus.clone(),
        is_first_call: AtomicBool::new(true),
        pre_model_hook: options.pre_model_hook,
        post_model_hook: options.post_model_hook,
        response_format: options.response_format,
        run_context: shared_run_context.clone(),
    };

    let mut tool_node = ToolNode::with_interceptors(executor, interceptor_chain)
        .with_parallel(options.parallel_tools);
    if let Some(ref bus) = options.event_bus {
        tool_node = tool_node.with_event_bus(bus.clone());
    }
    if let Some(ref store) = options.store {
        tool_node = tool_node.with_store(store.clone());
    }

    let mut builder = StateGraph::new()
        .add_node("agent", agent_node)
        .add_node("tools", tool_node)
        .set_entry_point("agent")
        .add_conditional_edges_with_path_map(
            "agent",
            |state: &MessageState| {
                if let Some(last) = state.last_message() {
                    if !last.tool_calls().is_empty() {
                        return "tools".to_string();
                    }
                }
                END.to_string()
            },
            HashMap::from([
                ("tools".to_string(), "tools".to_string()),
                (END.to_string(), END.to_string()),
            ]),
        )
        .add_edge("tools", "agent");

    if !options.interrupt_before.is_empty() {
        builder = builder.interrupt_before(options.interrupt_before);
    }
    if !options.interrupt_after.is_empty() {
        builder = builder.interrupt_after(options.interrupt_after);
    }

    let mut graph = builder.compile()?;
    // Wire the shared RunContext so invoke_with_context/stream_with_context
    // updates flow through to ChatModelNode.
    graph.run_context = shared_run_context;

    if let Some(max_iter) = options.max_iterations {
        graph = graph.with_max_iterations(max_iter);
    }

    // Auto-wire: explicit checkpointer > store-backed checkpointer > none
    let checkpointer: Option<Arc<dyn Checkpointer>> = match (&options.store, options.checkpointer) {
        (_, Some(ckpt)) => Some(ckpt),
        (Some(store), None) => Some(Arc::new(crate::StoreCheckpointer::new(store.clone()))),
        (None, None) => None,
    };

    if let Some(checkpointer) = checkpointer {
        graph = graph.with_checkpointer(checkpointer);
    }

    Ok(graph)
}

// ---------------------------------------------------------------------------
// Handoff tool — for multi-agent collaboration
// ---------------------------------------------------------------------------

struct HandoffTool {
    target_agent: String,
    tool_description: String,
}

#[async_trait]
impl Tool for HandoffTool {
    fn name(&self) -> &'static str {
        Box::leak(format!("transfer_to_{}", self.target_agent).into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.tool_description.clone().into_boxed_str())
    }

    async fn call(&self, _args: Value) -> Result<Value, SynapticError> {
        Ok(Value::String(format!(
            "Transferring to agent '{}'.",
            self.target_agent
        )))
    }
}

/// Create a handoff tool that signals transfer to another agent.
pub fn create_handoff_tool(agent_name: &str, description: &str) -> Arc<dyn Tool> {
    Arc::new(HandoffTool {
        target_agent: agent_name.to_string(),
        tool_description: description.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Supervisor — centralized multi-agent orchestration
// ---------------------------------------------------------------------------

/// Options for the supervisor multi-agent pattern.
#[derive(Default)]
pub struct SupervisorOptions {
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub store: Option<Arc<dyn Store>>,
    pub system_prompt: Option<String>,
}

/// A sub-agent node that invokes a compiled agent graph as a node.
struct SubAgentNode {
    graph: CompiledGraph<MessageState>,
}

#[async_trait]
impl Node<MessageState> for SubAgentNode {
    async fn process(
        &self,
        state: MessageState,
    ) -> Result<NodeOutput<MessageState>, SynapticError> {
        let result = self.graph.invoke(state).await?;
        Ok(result.into_state().into())
    }
}

/// Create a supervisor multi-agent graph.
#[traceable(skip = "model,agents,options")]
pub fn create_supervisor(
    model: Arc<dyn ChatModel>,
    agents: Vec<(String, CompiledGraph<MessageState>)>,
    options: SupervisorOptions,
) -> Result<CompiledGraph<MessageState>, SynapticError> {
    let agent_names: Vec<String> = agents.iter().map(|(name, _)| name.clone()).collect();

    // Create handoff tools for each agent
    let handoff_tools: Vec<Arc<dyn Tool>> = agent_names
        .iter()
        .map(|name| {
            create_handoff_tool(
                name,
                &format!("Transfer the conversation to the '{name}' agent."),
            )
        })
        .collect();

    let handoff_tool_defs: Vec<ToolDefinition> = handoff_tools
        .iter()
        .map(|t| ToolDefinition {
            name: t.name().to_string(),
            description: t.description().to_string(),
            input_schema: serde_json::json!({}),
            extras: None,
        })
        .collect();

    let default_prompt = format!(
        "You are a supervisor managing these agents: {}. \
         Use the transfer tools to delegate tasks to the appropriate agent. \
         When the task is complete, respond directly to the user.",
        agent_names.join(", ")
    );
    let system_prompt = options.system_prompt.unwrap_or(default_prompt);

    let supervisor_node = ChatModelNode {
        model,
        tool_defs: handoff_tool_defs.clone(),
        system_prompt: Some(system_prompt),
        interceptors: Arc::new(InterceptorChain::new(vec![])),
        event_bus: None,
        is_first_call: AtomicBool::new(false),
        pre_model_hook: None,
        post_model_hook: None,
        response_format: None,
        run_context: Arc::new(tokio::sync::RwLock::new(RunContext::default())),
    };

    let mut builder = StateGraph::new()
        .add_node("supervisor", supervisor_node)
        .set_entry_point("supervisor");

    for (name, graph) in agents {
        builder = builder
            .add_node(&name, SubAgentNode { graph })
            .add_edge(&name, "supervisor");
    }

    let agent_names_for_router = agent_names.clone();
    builder = builder.add_conditional_edges("supervisor", move |state: &MessageState| {
        if let Some(last) = state.last_message() {
            for tc in last.tool_calls() {
                for agent_name in &agent_names_for_router {
                    if tc.name == format!("transfer_to_{agent_name}") {
                        return agent_name.clone();
                    }
                }
            }
        }
        END.to_string()
    });

    let mut graph = builder.compile()?;

    if let Some(checkpointer) = options.checkpointer {
        graph = graph.with_checkpointer(checkpointer);
    }

    Ok(graph)
}

// ---------------------------------------------------------------------------
// Swarm — decentralized multi-agent collaboration
// ---------------------------------------------------------------------------

/// Options for the swarm multi-agent pattern.
#[derive(Default)]
pub struct SwarmOptions {
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub store: Option<Arc<dyn Store>>,
}

/// A swarm agent node: calls a model with its own tools + handoff tools.
struct SwarmAgentNode {
    model: Arc<dyn ChatModel>,
    tool_defs: Vec<ToolDefinition>,
    system_prompt: Option<String>,
}

#[async_trait]
impl Node<MessageState> for SwarmAgentNode {
    async fn process(
        &self,
        mut state: MessageState,
    ) -> Result<NodeOutput<MessageState>, SynapticError> {
        let mut messages = Vec::new();
        if let Some(ref prompt) = self.system_prompt {
            messages.push(Message::system(prompt));
        }
        messages.extend(state.messages.clone());

        let request = ChatRequest::new(messages).with_tools(self.tool_defs.clone());
        let response = self.model.chat(request).await?;
        state.messages.push(response.message);
        Ok(state.into())
    }
}

/// Swarm tool node: executes tool calls, but skips handoff tools.
struct SwarmToolNode {
    executor: SerialToolExecutor,
    handoff_tool_names: Vec<String>,
}

#[async_trait]
impl Node<MessageState> for SwarmToolNode {
    async fn process(
        &self,
        mut state: MessageState,
    ) -> Result<NodeOutput<MessageState>, SynapticError> {
        let last = state
            .last_message()
            .ok_or_else(|| SynapticError::Graph("no messages in state".to_string()))?;

        let tool_calls = last.tool_calls().to_vec();
        for call in &tool_calls {
            if self.handoff_tool_names.contains(&call.name) {
                state.messages.push(Message::tool(
                    "Transferring to agent.".to_string(),
                    &call.id,
                ));
            } else {
                let result = self
                    .executor
                    .execute(&call.name, call.arguments.clone())
                    .await?;
                state
                    .messages
                    .push(Message::tool(result.to_string(), &call.id));
            }
        }

        Ok(state.into())
    }
}

/// A swarm agent definition.
pub struct SwarmAgent {
    pub name: String,
    pub model: Arc<dyn ChatModel>,
    pub tools: Vec<Arc<dyn Tool>>,
    pub system_prompt: Option<String>,
}

/// Create a swarm multi-agent graph.
#[traceable(skip = "agents,options")]
pub fn create_swarm(
    agents: Vec<SwarmAgent>,
    options: SwarmOptions,
) -> Result<CompiledGraph<MessageState>, SynapticError> {
    if agents.is_empty() {
        return Err(SynapticError::Graph(
            "swarm requires at least one agent".to_string(),
        ));
    }

    let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
    let entry_agent = agent_names[0].clone();

    let all_handoff_tools: HashMap<String, Arc<dyn Tool>> = agent_names
        .iter()
        .map(|name| {
            (
                name.clone(),
                create_handoff_tool(
                    name,
                    &format!("Transfer the conversation to the '{name}' agent."),
                ),
            )
        })
        .collect();

    let handoff_tool_names: Vec<String> = all_handoff_tools
        .values()
        .map(|t| t.name().to_string())
        .collect();

    let mut builder = StateGraph::new();

    let global_registry = synaptic_tools::ToolRegistry::new();

    for agent in agents {
        let SwarmAgent {
            name,
            model,
            tools,
            system_prompt,
        } = agent;

        let mut tool_defs: Vec<ToolDefinition> = tools
            .iter()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: serde_json::json!({}),
                extras: None,
            })
            .collect();

        for tool in &tools {
            let _ = global_registry.register(tool.clone());
        }

        for other_name in &agent_names {
            if other_name != &name {
                if let Some(ht) = all_handoff_tools.get(other_name) {
                    tool_defs.push(ToolDefinition {
                        name: ht.name().to_string(),
                        description: ht.description().to_string(),
                        input_schema: serde_json::json!({}),
                        extras: None,
                    });
                }
            }
        }

        let agent_node = SwarmAgentNode {
            model,
            tool_defs,
            system_prompt,
        };

        builder = builder.add_node(&name, agent_node);
    }

    let executor = SerialToolExecutor::new(global_registry);
    let swarm_tool_node = SwarmToolNode {
        executor,
        handoff_tool_names: handoff_tool_names.clone(),
    };
    builder = builder.add_node("tools", swarm_tool_node);

    builder = builder.set_entry_point(&entry_agent);

    for agent_name in &agent_names {
        builder = builder.add_conditional_edges(agent_name, |state: &MessageState| {
            if let Some(last) = state.last_message() {
                if !last.tool_calls().is_empty() {
                    return "tools".to_string();
                }
            }
            END.to_string()
        });
    }

    let all_agent_names = agent_names.clone();
    builder = builder.add_conditional_edges("tools", move |state: &MessageState| {
        for msg in state.messages.iter().rev() {
            if msg.is_ai() && !msg.tool_calls().is_empty() {
                for tc in msg.tool_calls() {
                    for agent_name in &all_agent_names {
                        if tc.name == format!("transfer_to_{agent_name}") {
                            return agent_name.clone();
                        }
                    }
                }
                return all_agent_names[0].clone();
            }
        }
        all_agent_names[0].clone()
    });

    let mut graph = builder.compile()?;

    if let Some(checkpointer) = options.checkpointer {
        graph = graph.with_checkpointer(checkpointer);
    }

    Ok(graph)
}
