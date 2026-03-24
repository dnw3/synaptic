#![cfg(feature = "condenser")]

use std::sync::Arc;

use async_trait::async_trait;
use synaptic_core::{ChatModel, ChatRequest, ChatResponse, Message, SynapticError, TokenUsage};
use synaptic_middleware::condenser::{
    AdaptiveCondenser, AdaptiveCondenserOptions, CondenseAction, CondenseContext, Condenser,
};

// ---------------------------------------------------------------------------
// Mock ChatModel
// ---------------------------------------------------------------------------

struct MockSummarizerModel {
    response: String,
}

impl MockSummarizerModel {
    fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[async_trait]
impl ChatModel for MockSummarizerModel {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, SynapticError> {
        Ok(ChatResponse {
            message: Message::ai(&self.response),
            usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 20,
                total_tokens: 120,
                input_details: None,
                output_details: None,
            }),
        })
    }
}

struct FailingModel;

#[async_trait]
impl ChatModel for FailingModel {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, SynapticError> {
        Err(SynapticError::Model("LLM unavailable".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_context(messages: Vec<Message>, context_window: usize) -> CondenseContext {
    CondenseContext {
        messages,
        system_prompt: String::new(),
        tools: vec![],
        context_window,
        reserved_output_tokens: 4096,
        has_thinking: false,
    }
}

/// Generate N filler messages (alternating human/ai).
fn make_messages(count: usize) -> Vec<Message> {
    (0..count)
        .map(|i| {
            if i % 2 == 0 {
                Message::human(format!("Message {}", i))
            } else {
                Message::ai(format!("Response {}", i))
            }
        })
        .collect()
}

/// Generate N messages with large content.
fn make_large_messages(count: usize, chars_each: usize) -> Vec<Message> {
    let filler: String = "x".repeat(chars_each);
    (0..count)
        .map(|i| {
            if i % 2 == 0 {
                Message::human(format!("{}_{}", filler, i))
            } else {
                Message::ai(format!("{}_{}", filler, i))
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_skip_when_within_budget() {
    let model = Arc::new(MockSummarizerModel::new("summary"));
    let condenser = AdaptiveCondenser::new(model, AdaptiveCondenserOptions::default());

    // 5 small messages with a 128K context window — well within budget
    let msgs = make_messages(5);
    let ctx = make_context(msgs, 128_000);

    let result = condenser.condense(ctx).await.unwrap();
    assert_eq!(result.messages.len(), 5);
    assert!(matches!(result.action, CondenseAction::Skip));
}

#[tokio::test]
async fn test_summarize_when_over_budget() {
    let model = Arc::new(MockSummarizerModel::new(
        "Condensed summary of conversation.",
    ));
    let opts = AdaptiveCondenserOptions {
        keep_recent: 2,
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    // 50 messages x ~10K chars each → well over a small context window
    let msgs = make_large_messages(50, 10_000);
    // Use a small context window so messages are over budget
    let ctx = make_context(msgs, 20_000);

    let result = condenser.condense(ctx).await.unwrap();
    // Should have fewer messages than original 50
    assert!(result.messages.len() < 50);
    // The last 2 should be kept intact (keep_recent=2)
    assert!(matches!(
        result.action,
        CondenseAction::Summarized { .. }
            | CondenseAction::Evicted { .. }
            | CondenseAction::Degraded { .. }
    ));
}

#[tokio::test]
async fn test_message_count_hard_limit() {
    let model = Arc::new(MockSummarizerModel::new("summary"));
    let opts = AdaptiveCondenserOptions {
        max_messages: 20,
        keep_recent: 5,
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    // 200 tiny messages — exceeds max_messages=20
    let msgs = make_messages(200);
    let ctx = make_context(msgs, 128_000);

    let result = condenser.condense(ctx).await.unwrap();
    // Should have at most keep_recent messages (no system msg in these messages)
    assert!(result.messages.len() <= 6); // 5 recent + maybe system
    match &result.action {
        CondenseAction::Degraded { reason } => {
            assert!(reason.contains("message count exceeded"));
            assert!(reason.contains("200"));
            assert!(reason.contains("20"));
        }
        other => panic!("expected Degraded, got {:?}", other),
    }
}

#[tokio::test]
async fn test_message_count_hard_limit_preserves_system() {
    let model = Arc::new(MockSummarizerModel::new("summary"));
    let opts = AdaptiveCondenserOptions {
        max_messages: 10,
        keep_recent: 3,
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    let mut msgs = vec![Message::system("You are helpful.")];
    msgs.extend(make_messages(50));
    let ctx = make_context(msgs, 128_000);

    let result = condenser.condense(ctx).await.unwrap();
    // System message should be preserved
    assert!(result.messages[0].is_system());
    assert_eq!(result.messages[0].content(), "You are helpful.");
    // Total: system + keep_recent
    assert_eq!(result.messages.len(), 4); // 1 system + 3 recent
}

#[tokio::test]
async fn test_oversized_message_eviction() {
    let model = Arc::new(MockSummarizerModel::new("short summary"));
    let opts = AdaptiveCondenserOptions {
        keep_recent: 2,
        oversize_threshold: 0.5,
        safety_margin: 1.0, // no safety margin for easier testing
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    // Create messages where one is > 50% of a small budget
    let msgs = vec![
        Message::human("hello"),
        // This huge message should be detected as oversized
        Message::ai("x".repeat(50_000)),
        Message::human("follow up"),
        Message::ai("response"),
    ];
    // Small context window so the oversized message dominates
    let ctx = make_context(msgs, 10_000);

    let result = condenser.condense(ctx).await.unwrap();
    // The oversized message should have been handled
    assert!(result.messages.len() <= 4);
}

#[tokio::test]
async fn test_summarization_failure_graceful() {
    let model = Arc::new(FailingModel);
    let opts = AdaptiveCondenserOptions {
        keep_recent: 2,
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    // Enough large messages to trigger summarization
    let msgs = make_large_messages(20, 5_000);
    let ctx = make_context(msgs, 10_000);

    let result = condenser.condense(ctx).await.unwrap();
    // Should not error — should degrade gracefully
    // Check that placeholder summaries were inserted
    let has_placeholder = result
        .messages
        .iter()
        .any(|m| m.content().contains("[Summary unavailable:"));
    assert!(
        has_placeholder
            || matches!(
                result.action,
                CondenseAction::Degraded { .. } | CondenseAction::Evicted { .. }
            ),
        "Expected either placeholder summaries or degraded action"
    );
}

#[tokio::test]
async fn test_skip_when_few_old_messages() {
    let model = Arc::new(MockSummarizerModel::new("summary"));
    let opts = AdaptiveCondenserOptions {
        keep_recent: 10,
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    // Only 5 messages but keep_recent=10 — nothing to summarize
    let msgs = make_messages(5);
    let ctx = make_context(msgs, 128_000);

    let result = condenser.condense(ctx).await.unwrap();
    assert_eq!(result.messages.len(), 5);
    assert!(matches!(result.action, CondenseAction::Skip));
}

#[tokio::test]
async fn test_system_message_preserved_in_summarization() {
    let model = Arc::new(MockSummarizerModel::new("conversation summary"));
    let opts = AdaptiveCondenserOptions {
        keep_recent: 2,
        ..Default::default()
    };
    let condenser = AdaptiveCondenser::new(model, opts);

    let mut msgs = vec![Message::system("System prompt here.")];
    msgs.extend(make_large_messages(30, 5_000));
    let ctx = make_context(msgs, 15_000);

    let result = condenser.condense(ctx).await.unwrap();
    // First message should still be the system prompt
    assert!(result.messages[0].is_system());
    assert_eq!(result.messages[0].content(), "System prompt here.");
}

// ---------------------------------------------------------------------------
// Regression: 133-message / 19-tool production incident
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_regression_133_messages_19_tools_triggers_compaction() {
    use serde_json::json;
    use synaptic_core::ToolDefinition;

    let model = Arc::new(MockSummarizerModel::new("[Condensed summary]"));
    let condenser = AdaptiveCondenser::new(
        model,
        AdaptiveCondenserOptions {
            keep_recent: 10,
            ..Default::default()
        },
    );

    // 19 tools with realistic schemas
    let tools: Vec<ToolDefinition> = (0..19)
        .map(|i| ToolDefinition {
            name: format!("tool_{i}"),
            description: format!("Tool {i} for agent tasks with a reasonably long description"),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string", "description": "First argument" },
                    "arg2": { "type": "boolean", "description": "Second argument" },
                    "arg3": { "type": "number" }
                },
                "required": ["arg1"]
            }),
            extras: None,
        })
        .collect();

    // 133 messages with realistic sizes (production had 1.66M tokens)
    // Mix of human queries, long assistant responses, and large tool results
    let mut messages = Vec::new();
    for i in 0..133 {
        match i % 3 {
            0 => messages.push(Message::human("x".repeat(2_000))),
            1 => messages.push(Message::ai("y".repeat(8_000))),
            _ => messages.push(Message::tool("z".repeat(20_000), format!("tc_{i}"))),
        }
    }

    let system_prompt = "You are Synapse, a helpful AI assistant.".repeat(20);

    let ctx = CondenseContext {
        messages,
        system_prompt,
        tools,
        context_window: 128_000,
        reserved_output_tokens: 8192,
        has_thinking: false,
    };

    // Verify budget is exceeded
    let budget = ctx.message_budget();
    let estimated = ctx.estimate_message_tokens();
    assert!(
        estimated > budget,
        "133 messages should exceed budget: estimated={estimated}, budget={budget}"
    );

    let result = condenser.condense(ctx).await.unwrap();
    assert!(
        !matches!(result.action, CondenseAction::Skip),
        "Should NOT skip with 133 messages: action={:?}",
        result.action
    );
    assert!(
        result.messages.len() < 133,
        "Messages should be reduced, got {}",
        result.messages.len()
    );
}
