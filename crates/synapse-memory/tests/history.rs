use std::sync::Arc;

use synapse_core::{MemoryStore, Message, RunnableConfig, SynapseError};
use synapse_memory::{InMemoryStore, RunnableWithMessageHistory};
use synapse_runnables::Runnable;

/// A simple runnable that echoes the last human message content.
struct EchoRunnable;

#[async_trait::async_trait]
impl Runnable<Vec<Message>, String> for EchoRunnable {
    async fn invoke(
        &self,
        input: Vec<Message>,
        _config: &RunnableConfig,
    ) -> Result<String, SynapseError> {
        // Find the last human message and echo it
        let last_human = input
            .iter()
            .rev()
            .find(|m| m.is_human())
            .map(|m| format!("Echo: {}", m.content()))
            .unwrap_or_else(|| "No human message".to_string());
        Ok(last_human)
    }
}

#[tokio::test]
async fn runnable_with_message_history_loads_and_saves() {
    let store = Arc::new(InMemoryStore::new());
    let inner = EchoRunnable.boxed();
    let history = RunnableWithMessageHistory::new(inner, store.clone());

    let config = RunnableConfig::default().with_metadata(
        "session_id",
        serde_json::Value::String("test-session".to_string()),
    );

    // First invocation
    let output = history.invoke("hello".to_string(), &config).await.unwrap();
    assert_eq!(output, "Echo: hello");

    // Check that memory now has 2 messages (human + AI)
    let messages = store.load("test-session").await.unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages[0].is_human());
    assert_eq!(messages[0].content(), "hello");
    assert!(messages[1].is_ai());
    assert_eq!(messages[1].content(), "Echo: hello");

    // Second invocation should see the full history
    let output2 = history.invoke("world".to_string(), &config).await.unwrap();
    assert_eq!(output2, "Echo: world");

    // Now memory should have 4 messages
    let messages = store.load("test-session").await.unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[2].content(), "world");
    assert_eq!(messages[3].content(), "Echo: world");
}
