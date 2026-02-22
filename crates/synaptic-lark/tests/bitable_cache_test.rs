use synaptic_lark::{LarkBitableLlmCache, LarkConfig};

#[test]
fn constructor() {
    let cache = LarkBitableLlmCache::new(LarkConfig::new("cli", "secret"), "bascnXxx", "tblXxx");
    assert_eq!(cache.app_token(), "bascnXxx");
}

#[tokio::test]
#[ignore = "requires LARK_APP_ID and LARK_APP_SECRET"]
async fn integration_put_get_clear() {
    use synaptic_core::{ChatResponse, LlmCache, Message};
    let cache = LarkBitableLlmCache::new(
        LarkConfig::new(
            std::env::var("LARK_APP_ID").unwrap(),
            std::env::var("LARK_APP_SECRET").unwrap(),
        ),
        std::env::var("LARK_BITABLE_APP_TOKEN").unwrap(),
        std::env::var("LARK_BITABLE_TABLE_ID").unwrap(),
    );
    let key = "test-cache-key-001";
    cache.clear().await.unwrap();

    // Initially nothing in cache
    let result = cache.get(key).await.unwrap();
    assert!(result.is_none());

    // Put a response
    let response = ChatResponse {
        message: Message::ai("Hello from cache"),
        usage: None,
    };
    cache.put(key, &response).await.unwrap();

    // Now it should be retrievable
    let cached = cache.get(key).await.unwrap();
    assert!(cached.is_some());
    let cached = cached.unwrap();
    assert_eq!(cached.message.content(), "Hello from cache");

    // Clean up
    cache.clear().await.unwrap();
}
