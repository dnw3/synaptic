use std::sync::Arc;

use synapse_core::{ChatResponse, Message};
use synapse_eval::{Evaluator, LLMJudgeEvaluator};
use synapse_models::ScriptedChatModel;

#[tokio::test]
async fn llm_judge_high_score() {
    let model = Arc::new(ScriptedChatModel::new(vec![ChatResponse {
        message: Message::ai("8"),
        usage: None,
    }]));

    let evaluator = LLMJudgeEvaluator::new(model);
    let result = evaluator
        .evaluate("Paris", "Paris", "What is the capital of France?")
        .await
        .unwrap();

    assert!(result.passed);
    assert!((result.score - 0.8).abs() < 1e-6);
}

#[tokio::test]
async fn llm_judge_low_score() {
    let model = Arc::new(ScriptedChatModel::new(vec![ChatResponse {
        message: Message::ai("2"),
        usage: None,
    }]));

    let evaluator = LLMJudgeEvaluator::new(model);
    let result = evaluator
        .evaluate("Berlin", "Paris", "What is the capital of France?")
        .await
        .unwrap();

    assert!(!result.passed);
    assert!((result.score - 0.2).abs() < 1e-6);
}
