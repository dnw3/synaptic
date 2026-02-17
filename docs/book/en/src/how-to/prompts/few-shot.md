# Few-Shot Prompting

`FewShotChatMessagePromptTemplate` injects example conversations into a prompt for few-shot learning. Each example is a pair of human input and AI output, formatted as alternating `Human` and `AI` messages. An optional system prefix message can be prepended.

## Basic Usage

Create the template with a list of `FewShotExample` values and a suffix `PromptTemplate` for the user's actual query:

```rust
use std::collections::HashMap;
use synapse_prompts::{
    FewShotChatMessagePromptTemplate, FewShotExample, PromptTemplate,
};

let template = FewShotChatMessagePromptTemplate::new(
    vec![
        FewShotExample {
            input: "What is 2+2?".to_string(),
            output: "4".to_string(),
        },
        FewShotExample {
            input: "What is 3+3?".to_string(),
            output: "6".to_string(),
        },
    ],
    PromptTemplate::new("{{ question }}"),
);

let values = HashMap::from([
    ("question".to_string(), "What is 4+4?".to_string()),
]);
let messages = template.format(&values).unwrap();

// messages[0] => Human("What is 2+2?")  -- example 1 input
// messages[1] => AI("4")                 -- example 1 output
// messages[2] => Human("What is 3+3?")  -- example 2 input
// messages[3] => AI("6")                 -- example 2 output
// messages[4] => Human("What is 4+4?")  -- actual query (suffix)
```

Each `FewShotExample` has two fields:

- `input` -- the human message for this example
- `output` -- the AI response for this example

The `suffix` template is rendered with the user-provided variables and appended as the final human message.

## Adding a System Prefix

Use `with_prefix()` to prepend a system message before the examples:

```rust
use std::collections::HashMap;
use synapse_prompts::{
    FewShotChatMessagePromptTemplate, FewShotExample, PromptTemplate,
};

let template = FewShotChatMessagePromptTemplate::new(
    vec![FewShotExample {
        input: "hi".to_string(),
        output: "hello".to_string(),
    }],
    PromptTemplate::new("{{ input }}"),
)
.with_prefix(PromptTemplate::new("You are a polite assistant."));

let values = HashMap::from([("input".to_string(), "hey".to_string())]);
let messages = template.format(&values).unwrap();

// messages[0] => System("You are a polite assistant.")  -- prefix
// messages[1] => Human("hi")                            -- example input
// messages[2] => AI("hello")                            -- example output
// messages[3] => Human("hey")                           -- actual query
```

The prefix template supports `{{ variable }}` interpolation, so you can parameterize the system message too.

## Using as a Runnable

`FewShotChatMessagePromptTemplate` implements `Runnable<HashMap<String, String>, Vec<Message>>`, so you can call `invoke()` or compose it in pipelines:

```rust
use std::collections::HashMap;
use synapse_core::RunnableConfig;
use synapse_prompts::{
    FewShotChatMessagePromptTemplate, FewShotExample, PromptTemplate,
};
use synapse_runnables::Runnable;

let template = FewShotChatMessagePromptTemplate::new(
    vec![FewShotExample {
        input: "x".to_string(),
        output: "y".to_string(),
    }],
    PromptTemplate::new("{{ q }}"),
);

let config = RunnableConfig::default();
let values = HashMap::from([("q".to_string(), "z".to_string())]);
let messages = template.invoke(values, &config).await?;
// 3 messages: Human("x"), AI("y"), Human("z")
```

> **Note:** The `Runnable` implementation for `FewShotChatMessagePromptTemplate` takes `HashMap<String, String>`, while `ChatPromptTemplate` takes `HashMap<String, serde_json::Value>`. This difference reflects their underlying template rendering: few-shot templates use `PromptTemplate::render()` which works with string values.
