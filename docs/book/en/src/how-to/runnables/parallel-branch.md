# Parallel & Branch

This guide shows how to run multiple runnables concurrently with `RunnableParallel` and how to route input to different runnables with `RunnableBranch`.

## RunnableParallel

`RunnableParallel` runs named branches concurrently on the same input, then merges all outputs into a single `serde_json::Value` object keyed by branch name.

The input type must implement `Clone`, because each branch receives its own copy. Every branch must produce a `serde_json::Value` output.

### Basic usage

```rust
use serde_json::Value;
use synaptic_runnables::{Runnable, RunnableParallel, RunnableLambda};
use synaptic_core::RunnableConfig;

let parallel = RunnableParallel::new(vec![
    (
        "upper".to_string(),
        RunnableLambda::new(|x: String| async move {
            Ok(Value::String(x.to_uppercase()))
        }).boxed(),
    ),
    (
        "lower".to_string(),
        RunnableLambda::new(|x: String| async move {
            Ok(Value::String(x.to_lowercase()))
        }).boxed(),
    ),
    (
        "length".to_string(),
        RunnableLambda::new(|x: String| async move {
            Ok(Value::Number(x.len().into()))
        }).boxed(),
    ),
]);

let config = RunnableConfig::default();
let result = parallel.invoke("Hello".to_string(), &config).await?;

// result is a JSON object:
// {"upper": "HELLO", "lower": "hello", "length": 5}
assert_eq!(result["upper"], "HELLO");
assert_eq!(result["lower"], "hello");
assert_eq!(result["length"], 5);
```

### Constructor

`RunnableParallel::new()` takes a `Vec<(String, BoxRunnable<I, Value>)>` -- a list of `(name, runnable)` pairs. All branches run concurrently via `futures::future::join_all`.

### In a chain

`RunnableParallel` implements `Runnable<I, Value>`, so you can use it in a pipe chain. A common pattern is to fan out processing and then merge the results:

```rust
let analyze = RunnableParallel::new(vec![
    ("summary".to_string(), summarizer.boxed()),
    ("keywords".to_string(), keyword_extractor.boxed()),
]);

let format_report = RunnableLambda::new(|data: Value| async move {
    Ok(format!(
        "Summary: {}\nKeywords: {}",
        data["summary"], data["keywords"]
    ))
});

let chain = analyze.boxed() | format_report.boxed();
```

### Error handling

If any branch fails, the entire `RunnableParallel` invocation returns the first error encountered. Successful branches that completed before the failure are discarded.

---

## RunnableBranch

`RunnableBranch` routes input to one of several runnables based on condition functions. It evaluates conditions in order, invoking the runnable associated with the first matching condition. If no conditions match, the default runnable is used.

### Basic usage

```rust
use synaptic_runnables::{Runnable, RunnableBranch, RunnableLambda, BoxRunnable};
use synaptic_core::RunnableConfig;

let branch = RunnableBranch::new(
    vec![
        (
            Box::new(|x: &String| x.starts_with("hi")) as Box<dyn Fn(&String) -> bool + Send + Sync>,
            RunnableLambda::new(|x: String| async move {
                Ok(format!("Greeting: {x}"))
            }).boxed(),
        ),
        (
            Box::new(|x: &String| x.starts_with("bye")),
            RunnableLambda::new(|x: String| async move {
                Ok(format!("Farewell: {x}"))
            }).boxed(),
        ),
    ],
    // Default: used when no condition matches
    RunnableLambda::new(|x: String| async move {
        Ok(format!("Other: {x}"))
    }).boxed(),
);

let config = RunnableConfig::default();

let r1 = branch.invoke("hi there".to_string(), &config).await?;
assert_eq!(r1, "Greeting: hi there");

let r2 = branch.invoke("bye now".to_string(), &config).await?;
assert_eq!(r2, "Farewell: bye now");

let r3 = branch.invoke("something else".to_string(), &config).await?;
assert_eq!(r3, "Other: something else");
```

### Constructor

`RunnableBranch::new()` takes two arguments:

1. `branches: Vec<(BranchCondition<I>, BoxRunnable<I, O>)>` -- condition/runnable pairs evaluated in order. The condition type is `Box<dyn Fn(&I) -> bool + Send + Sync>`.
2. `default: BoxRunnable<I, O>` -- the fallback runnable when no condition matches.

### In a chain

`RunnableBranch` implements `Runnable<I, O>`, so it works with the pipe operator:

```rust
let preprocess = RunnableLambda::new(|x: String| async move {
    Ok(x.trim().to_string())
});

let route = RunnableBranch::new(
    vec![/* conditions */],
    default_handler.boxed(),
);

let chain = preprocess.boxed() | route.boxed();
```

### When to use each

- Use **`RunnableParallel`** when you need to run multiple operations on the same input concurrently and combine all results.
- Use **`RunnableBranch`** when you need to select a single processing path based on the input value.
