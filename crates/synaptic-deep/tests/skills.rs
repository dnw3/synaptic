use async_trait::async_trait;
use std::sync::Arc;
use synaptic_core::{Message, SynapticError};
use synaptic_deep::backend::{Backend, StateBackend};
use synaptic_deep::middleware::skills::SkillsMiddleware;
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};

/// A mock ModelCaller that captures the request for inspection.
struct CapturingCaller;

#[async_trait]
impl ModelCaller for CapturingCaller {
    async fn call(&self, request: ModelRequest) -> Result<ModelResponse, SynapticError> {
        // Return a response that includes the system prompt so tests can inspect it.
        let content = request.system_prompt.unwrap_or_default();
        Ok(ModelResponse {
            message: Message::ai(content),
            usage: None,
        })
    }
}

fn empty_request() -> ModelRequest {
    ModelRequest {
        messages: vec![Message::human("hello")],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    }
}

#[tokio::test]
async fn no_skills_no_injection() {
    let backend = Arc::new(StateBackend::new());
    let mw = SkillsMiddleware::new(backend, ".skills".to_string());
    let caller = CapturingCaller;
    let request = empty_request();
    let response = mw.wrap_model_call(request, &caller).await.unwrap();
    // No skills discovered, so system prompt should be empty
    assert!(response.message.content().is_empty());
}

#[tokio::test]
async fn discovers_skills_from_frontmatter() {
    let backend = Arc::new(StateBackend::new());
    // Create a skill directory structure
    backend
        .write_file(
            ".skills/search/SKILL.md",
            "---\nname: search\ndescription: Search the web\n---\n# Search\nDetails here.",
        )
        .await
        .unwrap();
    backend
        .write_file(
            ".skills/code/SKILL.md",
            "---\nname: code-review\ndescription: Review code for issues\n---\n# Code Review",
        )
        .await
        .unwrap();

    let mw = SkillsMiddleware::new(backend, ".skills".to_string());
    let caller = CapturingCaller;
    let request = empty_request();
    let response = mw.wrap_model_call(request, &caller).await.unwrap();

    let prompt = response.message.content().to_string();
    assert!(prompt.contains("<available_skills>"));
    assert!(prompt.contains("search"));
    assert!(prompt.contains("code-review"));
    assert!(prompt.contains("Search the web"));
}

#[tokio::test]
async fn appends_to_existing_system_prompt() {
    let backend = Arc::new(StateBackend::new());
    backend
        .write_file(
            ".skills/tool/SKILL.md",
            "---\nname: my-tool\ndescription: A tool\n---\n",
        )
        .await
        .unwrap();

    let mw = SkillsMiddleware::new(backend, ".skills".to_string());
    let caller = CapturingCaller;
    let mut request = empty_request();
    request.system_prompt = Some("You are helpful.".to_string());
    let response = mw.wrap_model_call(request, &caller).await.unwrap();

    let prompt = response.message.content().to_string();
    assert!(prompt.starts_with("You are helpful."));
    assert!(prompt.contains("my-tool"));
}

#[tokio::test]
async fn invalid_frontmatter_skipped() {
    let backend = Arc::new(StateBackend::new());
    // No frontmatter
    backend
        .write_file(
            ".skills/bad/SKILL.md",
            "# Just a header\nNo frontmatter here.",
        )
        .await
        .unwrap();

    let mw = SkillsMiddleware::new(backend, ".skills".to_string());
    let caller = CapturingCaller;
    let request = empty_request();
    let response = mw.wrap_model_call(request, &caller).await.unwrap();
    // No valid skills, so system prompt should be empty
    assert!(response.message.content().is_empty());
}
