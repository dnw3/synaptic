use serde::Deserialize;
use synapse_core::RunnableConfig;
use synapse_parsers::StructuredOutputParser;
use synapse_runnables::Runnable;

#[derive(Debug, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
}

#[tokio::test]
async fn deserializes_into_struct() {
    let parser = StructuredOutputParser::<Person>::new();
    let config = RunnableConfig::default();
    let result = parser
        .invoke(r#"{"name": "Bob", "age": 25}"#.to_string(), &config)
        .await
        .unwrap();
    assert_eq!(
        result,
        Person {
            name: "Bob".to_string(),
            age: 25
        }
    );
}

#[tokio::test]
async fn returns_error_on_wrong_shape() {
    let parser = StructuredOutputParser::<Person>::new();
    let config = RunnableConfig::default();
    let err = parser
        .invoke(r#"{"name": "Bob"}"#.to_string(), &config)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("structured parse error"));
}
