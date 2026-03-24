#![cfg(feature = "condenser")]

use std::sync::Arc;

use synaptic_core::Message;
use synaptic_middleware::condenser::{
    CondenseAction, CondenseContext, Condenser, CondenserMiddleware, DefaultContextWindowResolver,
    NoOpCondenser,
};
use synaptic_middleware::{Interceptor, ModelRequest};

fn make_context(messages: Vec<Message>) -> CondenseContext {
    CondenseContext {
        messages,
        system_prompt: String::new(),
        tools: vec![],
        context_window: 128_000,
        reserved_output_tokens: 4096,
        has_thinking: false,
    }
}

#[tokio::test]
async fn noop_unchanged() {
    let c = NoOpCondenser;
    let msgs = vec![Message::human("hi"), Message::ai("hello")];
    let result = c.condense(make_context(msgs)).await.unwrap();
    assert_eq!(result.messages.len(), 2);
    assert_eq!(result.messages[0].content(), "hi");
    assert!(matches!(result.action, CondenseAction::Skip));
}

#[tokio::test]
async fn noop_estimates_tokens() {
    let c = NoOpCondenser;
    let msgs = vec![Message::human("hello world")];
    let result = c.condense(make_context(msgs)).await.unwrap();
    assert!(result.estimated_tokens > 0);
}

#[tokio::test]
async fn middleware_applies() {
    let condenser = Arc::new(NoOpCondenser);
    let resolver = Arc::new(DefaultContextWindowResolver::new(128_000));
    let mw = CondenserMiddleware::new(condenser, resolver, 4096);

    let mut request = ModelRequest {
        messages: vec![Message::human("1"), Message::ai("2"), Message::human("3")],
        tools: vec![],
        tool_choice: None,
        system_prompt: None,
        thinking: None,
    };

    mw.before_model(&mut request).await.unwrap();
    // NoOp leaves all messages intact
    assert_eq!(request.messages.len(), 3);
}

#[tokio::test]
async fn condense_context_budget_calculation() {
    let ctx = CondenseContext {
        messages: vec![Message::human("hello")],
        system_prompt: "You are helpful.".to_string(),
        tools: vec![],
        context_window: 100_000,
        reserved_output_tokens: 4096,
        has_thinking: false,
    };

    let budget = ctx.message_budget();
    // budget = 100_000 - 4096 - system_tokens - tools_tokens - 64
    assert!(budget < 100_000);
    assert!(budget > 90_000);
}

#[tokio::test]
async fn condense_context_thinking_reserve() {
    let ctx = CondenseContext {
        messages: vec![],
        system_prompt: String::new(),
        tools: vec![],
        context_window: 200_000,
        reserved_output_tokens: 4096,
        has_thinking: true,
    };

    // With thinking, effective_output_reserve includes THINKING_OUTPUT_RESERVE (32_000)
    assert_eq!(ctx.effective_output_reserve(), 4096 + 32_000);
}

#[tokio::test]
async fn default_resolver_fallback() {
    use synaptic_middleware::condenser::ContextWindowResolver;

    let resolver = DefaultContextWindowResolver::new(128_000);
    assert_eq!(
        resolver.resolve("unknown-model", "unknown-provider"),
        128_000
    );

    resolver.register("gpt-4o", "openai", 200_000);
    assert_eq!(resolver.resolve("gpt-4o", "openai"), 200_000);
    // Unknown still falls back
    assert_eq!(resolver.resolve("other", "openai"), 128_000);
}
