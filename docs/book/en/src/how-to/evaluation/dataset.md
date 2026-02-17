# Datasets

The `Dataset` type and `evaluate()` function provide a batch evaluation pipeline. You define a dataset of input-reference pairs, generate predictions, and score them all at once to produce an `EvalReport`.

## Creating a Dataset

A `Dataset` is a collection of `DatasetItem` values, each with an `input` and a `reference` (expected answer):

```rust
use synapse_eval::{Dataset, DatasetItem};

// From DatasetItem structs
let dataset = Dataset::new(vec![
    DatasetItem {
        input: "What is 2+2?".to_string(),
        reference: "4".to_string(),
    },
    DatasetItem {
        input: "Capital of France?".to_string(),
        reference: "Paris".to_string(),
    },
]);

// From string pairs (convenience method)
let dataset = Dataset::from_pairs(vec![
    ("What is 2+2?", "4"),
    ("Capital of France?", "Paris"),
]);
```

## Running Batch Evaluation

The `evaluate()` function takes an evaluator, a dataset, and a slice of predictions. It evaluates each prediction against the corresponding dataset item and returns an `EvalReport`:

```rust
use synapse_eval::{evaluate, Dataset, ExactMatchEvaluator};

let dataset = Dataset::from_pairs(vec![
    ("What is 2+2?", "4"),
    ("Capital of France?", "Paris"),
    ("Largest ocean?", "Pacific"),
]);

let evaluator = ExactMatchEvaluator::new();

// Your model's predictions (one per dataset item)
let predictions = vec![
    "4".to_string(),
    "Paris".to_string(),
    "Atlantic".to_string(),  // Wrong!
];

let report = evaluate(&evaluator, &dataset, &predictions).await?;

println!("Total: {}", report.total);      // 3
println!("Passed: {}", report.passed);     // 2
println!("Accuracy: {:.0}%", report.accuracy * 100.0);  // 67%
```

The number of predictions must match the number of dataset items. If they differ, `evaluate()` returns a `SynapseError::Validation`.

## `EvalReport`

The report contains aggregate statistics and per-item results:

```rust
pub struct EvalReport {
    pub total: usize,
    pub passed: usize,
    pub accuracy: f32,
    pub results: Vec<EvalResult>,
}
```

You can inspect individual results for detailed feedback:

```rust
for (i, result) in report.results.iter().enumerate() {
    let status = if result.passed { "PASS" } else { "FAIL" };
    let reason = result.reasoning.as_deref().unwrap_or("--");
    println!("[{status}] Item {i}: score={:.2}, reason={reason}", result.score);
}
```

## End-to-End Example

A typical evaluation workflow:

1. Build a dataset of test cases.
2. Run your model/chain on each input to produce predictions.
3. Score predictions with an evaluator.
4. Inspect the report.

```rust
use synapse_eval::{evaluate, Dataset, ExactMatchEvaluator};

// 1. Dataset
let dataset = Dataset::from_pairs(vec![
    ("2+2", "4"),
    ("3*5", "15"),
    ("10/2", "5"),
]);

// 2. Generate predictions (in practice, run your model)
let predictions: Vec<String> = dataset.items.iter()
    .map(|item| {
        // Simulated model output
        match item.input.as_str() {
            "2+2" => "4",
            "3*5" => "15",
            "10/2" => "5",
            _ => "unknown",
        }.to_string()
    })
    .collect();

// 3. Evaluate
let evaluator = ExactMatchEvaluator::new();
let report = evaluate(&evaluator, &dataset, &predictions).await?;

// 4. Report
println!("Accuracy: {:.0}% ({}/{})",
    report.accuracy * 100.0, report.passed, report.total);
```

## Using Different Evaluators

The `evaluate()` function works with any `Evaluator`. Swap in a different evaluator to change the scoring criteria without modifying the dataset or prediction pipeline:

```rust
use synapse_eval::{evaluate, RegexMatchEvaluator};

// Check that predictions contain a date
let evaluator = RegexMatchEvaluator::new(r"\d{4}-\d{2}-\d{2}")?;
let report = evaluate(&evaluator, &dataset, &predictions).await?;
```
