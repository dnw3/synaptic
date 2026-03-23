use std::sync::Arc;
use synaptic_core::{ChatModel, ChatRequest, ChatResponse, Message, SynapticError};
use synaptic_deep::backend::StateBackend;
use synaptic_deep::{create_deep_agent, DeepAgentOptions};
use synaptic_graph::MessageState;

/// A model that always returns a plain AI text response (no tool calls).
struct FinalAnswerModel;

#[async_trait::async_trait]
impl ChatModel for FinalAnswerModel {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, SynapticError> {
        Ok(ChatResponse {
            message: Message::ai("Done"),
            usage: None,
        })
    }
}

#[tokio::test]
async fn minimal_offline_agent() {
    let backend = Arc::new(StateBackend::new());
    let model: Arc<dyn ChatModel> = Arc::new(FinalAnswerModel);
    let mut options = DeepAgentOptions::new(backend);
    options.subagent.enable_subagents = false;
    options.skills.enable_skills = false;
    options.context.enable_memory = false;

    let agent = create_deep_agent(model, options).unwrap();
    let state = MessageState::with_messages(vec![Message::human("hello")]);
    let result = agent.invoke(state).await.unwrap().into_state();

    let last = result.last_message().unwrap();
    assert!(last.is_ai());
    assert_eq!(last.content(), "Done");
}

#[tokio::test]
async fn filesystem_disabled_still_works() {
    let backend = Arc::new(StateBackend::new());
    let model: Arc<dyn ChatModel> = Arc::new(FinalAnswerModel);
    let mut options = DeepAgentOptions::new(backend);
    options.filesystem.enable_filesystem = false;
    options.subagent.enable_subagents = false;
    options.skills.enable_skills = false;
    options.context.enable_memory = false;

    let agent = create_deep_agent(model, options).unwrap();
    let state = MessageState::with_messages(vec![Message::human("hello")]);
    let result = agent.invoke(state).await.unwrap().into_state();

    assert!(result.last_message().unwrap().is_ai());
}

#[tokio::test]
async fn custom_system_prompt_accepted() {
    let backend = Arc::new(StateBackend::new());
    let model: Arc<dyn ChatModel> = Arc::new(FinalAnswerModel);
    let mut options = DeepAgentOptions::new(backend);
    options.context.system_prompt = Some("You are a test agent.".to_string());
    options.subagent.enable_subagents = false;
    options.skills.enable_skills = false;
    options.context.enable_memory = false;

    let agent = create_deep_agent(model, options).unwrap();
    let state = MessageState::with_messages(vec![Message::human("hello")]);
    let result = agent.invoke(state).await.unwrap().into_state();

    assert!(result.last_message().unwrap().is_ai());
}

#[test]
fn options_default_values() {
    let backend = Arc::new(StateBackend::new());
    let options = DeepAgentOptions::new(backend);

    assert!(options.subagent.enable_subagents);
    assert!(options.filesystem.enable_filesystem);
    assert!(options.skills.enable_skills);
    assert!(options.context.enable_memory);
    assert!(options.context.system_prompt.is_none());
    assert!(options.tools.is_empty());
    assert!(options.interceptors.is_empty());
    assert_eq!(options.condenser.max_input_tokens, 128_000);
    assert!((options.condenser.summarization_threshold - 0.85).abs() < 0.01);
    assert_eq!(options.condenser.eviction_threshold, 20_000);
    assert_eq!(options.subagent.max_subagent_depth, 3);
    assert_eq!(
        options.skills.skills_dirs,
        vec![".claude/skills".to_string()]
    );
    assert_eq!(options.context.memory_file, Some("AGENTS.md".to_string()));
    assert!(options.subagent.subagents.is_empty());
    assert!(options.checkpointer.is_none());
    assert!(options.store.is_none());
}

#[tokio::test]
async fn all_features_disabled_produces_basic_agent() {
    let backend = Arc::new(StateBackend::new());
    let model: Arc<dyn ChatModel> = Arc::new(FinalAnswerModel);
    let mut options = DeepAgentOptions::new(backend);
    options.filesystem.enable_filesystem = false;
    options.subagent.enable_subagents = false;
    options.skills.enable_skills = false;
    options.context.enable_memory = false;

    let agent = create_deep_agent(model, options).unwrap();
    let state = MessageState::with_messages(vec![Message::human("hello")]);
    let result = agent.invoke(state).await.unwrap().into_state();

    assert!(!result.messages.is_empty());
    // Should have human + AI messages
    assert!(result.messages.iter().any(|m| m.is_human()));
    assert!(result.messages.iter().any(|m| m.is_ai()));
}

#[test]
fn state_backend_new_and_default() {
    let backend = StateBackend::new();
    let _arc = Arc::new(backend);

    // Default trait also works
    let backend2 = StateBackend::default();
    let _arc2 = Arc::new(backend2);
}

#[tokio::test]
async fn custom_options_fields_mutated() {
    let backend = Arc::new(StateBackend::new());
    let mut options = DeepAgentOptions::new(backend.clone());

    options.condenser.max_input_tokens = 50_000;
    options.condenser.summarization_threshold = 0.7;
    options.condenser.eviction_threshold = 5_000;
    options.subagent.max_subagent_depth = 1;
    options.skills.skills_dirs = vec![];
    options.context.memory_file = None;
    options.subagent.enable_subagents = false;
    options.skills.enable_skills = false;
    options.context.enable_memory = false;

    assert_eq!(options.condenser.max_input_tokens, 50_000);
    assert!((options.condenser.summarization_threshold - 0.7).abs() < 0.01);
    assert_eq!(options.condenser.eviction_threshold, 5_000);
    assert_eq!(options.subagent.max_subagent_depth, 1);
    assert!(options.skills.skills_dirs.is_empty());
    assert!(options.context.memory_file.is_none());

    // Should still compile into a working agent
    let model: Arc<dyn ChatModel> = Arc::new(FinalAnswerModel);
    let agent = create_deep_agent(model, options).unwrap();
    let state = MessageState::with_messages(vec![Message::human("hi")]);
    let result = agent.invoke(state).await.unwrap().into_state();
    assert!(result.last_message().unwrap().is_ai());
}
