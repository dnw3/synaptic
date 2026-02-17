# Evaluation

Synapse provides an evaluation framework for measuring the quality of AI outputs. The `Evaluator` trait defines a standard interface for scoring predictions against references, and the `Dataset` + `evaluate()` pipeline makes it easy to run batch evaluations across many test cases.

## The `Evaluator` Trait

All evaluators implement the `Evaluator` trait from `synapse_eval`:

```rust
#[async_trait]
pub trait Evaluator: Send + Sync {
    async fn evaluate(
        &self,
        prediction: &str,
        reference: &str,
        input: &str,
    ) -> Result<EvalResult, SynapseError>;
}
```

- **`prediction`** -- the AI's output to evaluate.
- **`reference`** -- the expected or ground-truth answer.
- **`input`** -- the original input that produced the prediction.

## `EvalResult`

Every evaluator returns an `EvalResult`:

```rust
pub struct EvalResult {
    pub score: f64,       // Between 0.0 and 1.0
    pub passed: bool,     // true if score >= 0.5
    pub reasoning: Option<String>,  // Optional explanation
}
```

Helper constructors:

| Method | Score | Passed |
|--------|-------|--------|
| `EvalResult::pass()` | 1.0 | true |
| `EvalResult::fail()` | 0.0 | false |
| `EvalResult::with_score(0.75)` | 0.75 | true (>= 0.5) |

You can attach reasoning with `.with_reasoning("explanation")`.

## Built-in Evaluators

Synapse provides five evaluators out of the box:

| Evaluator | What It Checks |
|-----------|----------------|
| `ExactMatchEvaluator` | Exact string equality (with optional case-insensitive mode) |
| `JsonValidityEvaluator` | Whether the prediction is valid JSON |
| `RegexMatchEvaluator` | Whether the prediction matches a regex pattern |
| `EmbeddingDistanceEvaluator` | Cosine similarity between prediction and reference embeddings |
| `LLMJudgeEvaluator` | Uses an LLM to score prediction quality on a 0-10 scale |

See [Evaluators](evaluators.md) for detailed usage of each.

## Batch Evaluation

The `evaluate()` function runs an evaluator across a `Dataset` of test cases, producing an `EvalReport` with aggregate statistics. See [Datasets](dataset.md) for details.

## Guides

- [Evaluators](evaluators.md) -- usage and configuration for each built-in evaluator
- [Datasets](dataset.md) -- batch evaluation with `Dataset` and `evaluate()`
