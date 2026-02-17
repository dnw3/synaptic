# Assign & Pick

This guide shows how to use `RunnableAssign` to merge computed values into a JSON object and `RunnablePick` to extract specific keys from one.

## RunnableAssign

`RunnableAssign` takes a JSON object as input, runs named branches in parallel on that object, and merges the branch outputs back into the original object. This is useful for enriching data as it flows through a chain -- you keep the original fields and add new computed ones.

### Basic usage

```rust
use serde_json::{json, Value};
use synaptic_runnables::{Runnable, RunnableAssign, RunnableLambda};
use synaptic_core::RunnableConfig;

let assign = RunnableAssign::new(vec![
    (
        "name_upper".to_string(),
        RunnableLambda::new(|input: Value| async move {
            let name = input["name"].as_str().unwrap_or_default();
            Ok(Value::String(name.to_uppercase()))
        }).boxed(),
    ),
    (
        "greeting".to_string(),
        RunnableLambda::new(|input: Value| async move {
            let name = input["name"].as_str().unwrap_or_default();
            Ok(Value::String(format!("Hello, {name}!")))
        }).boxed(),
    ),
]);

let config = RunnableConfig::default();
let input = json!({"name": "Alice", "age": 30});
let result = assign.invoke(input, &config).await?;

// Original fields are preserved, new fields are merged in
assert_eq!(result["name"], "Alice");
assert_eq!(result["age"], 30);
assert_eq!(result["name_upper"], "ALICE");
assert_eq!(result["greeting"], "Hello, Alice!");
```

### How it works

1. The input must be a JSON object (`Value::Object`). If it is not, `RunnableAssign` returns a `SynapticError::Validation` error.
2. Each branch receives a clone of the full input object.
3. All branches run concurrently via `futures::future::join_all`.
4. Branch outputs are inserted into the original object using the branch name as the key. If a branch name collides with an existing key, the branch output overwrites the original value.

### Constructor

`RunnableAssign::new()` takes a `Vec<(String, BoxRunnable<Value, Value>)>` -- named branches that each transform the input into a value to be merged.

### Shorthand via `RunnablePassthrough`

`RunnablePassthrough` provides a convenience method that creates a `RunnableAssign` directly:

```rust
use synaptic_runnables::{RunnablePassthrough, RunnableLambda};
use serde_json::Value;

let assign = RunnablePassthrough::assign(vec![
    (
        "processed".to_string(),
        RunnableLambda::new(|input: Value| async move {
            // compute something from the input
            Ok(Value::String("result".to_string()))
        }).boxed(),
    ),
]);
```

---

## RunnablePick

`RunnablePick` extracts specified keys from a JSON object, producing a new object containing only those keys. Keys that do not exist in the input are silently omitted from the output.

### Basic usage

```rust
use serde_json::{json, Value};
use synaptic_runnables::{Runnable, RunnablePick};
use synaptic_core::RunnableConfig;

let pick = RunnablePick::new(vec![
    "name".to_string(),
    "age".to_string(),
]);

let config = RunnableConfig::default();
let input = json!({
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com",
    "internal_id": 42
});

let result = pick.invoke(input, &config).await?;

// Only the picked keys are present
assert_eq!(result, json!({"name": "Alice", "age": 30}));
```

### Error handling

`RunnablePick` expects a JSON object as input. If the input is not an object (e.g., a string or array), it returns a `SynapticError::Validation` error.

Missing keys are not an error -- they are simply absent from the output:

```rust
let pick = RunnablePick::new(vec!["name".to_string(), "missing_key".to_string()]);
let result = pick.invoke(json!({"name": "Bob"}), &config).await?;
assert_eq!(result, json!({"name": "Bob"}));
```

---

## Combining Assign and Pick in a chain

A common pattern is to use `RunnableAssign` to enrich data, then `RunnablePick` to select only the fields needed downstream:

```rust
use serde_json::{json, Value};
use synaptic_runnables::{Runnable, RunnableAssign, RunnablePick, RunnableLambda};
use synaptic_core::RunnableConfig;

// Step 1: Enrich input with a computed field
let assign = RunnableAssign::new(vec![
    (
        "full_name".to_string(),
        RunnableLambda::new(|input: Value| async move {
            let first = input["first"].as_str().unwrap_or_default();
            let last = input["last"].as_str().unwrap_or_default();
            Ok(Value::String(format!("{first} {last}")))
        }).boxed(),
    ),
]);

// Step 2: Pick only what the next step needs
let pick = RunnablePick::new(vec!["full_name".to_string()]);

let chain = assign.boxed() | pick.boxed();

let config = RunnableConfig::default();
let input = json!({"first": "Jane", "last": "Doe", "internal_id": 99});
let result = chain.invoke(input, &config).await?;

assert_eq!(result, json!({"full_name": "Jane Doe"}));
```
