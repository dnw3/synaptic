use synaptic_lark::{LarkBitableMemoryStore, LarkConfig};

#[test]
fn constructor_accepts_config() {
    let store = LarkBitableMemoryStore::new(LarkConfig::new("cli", "secret"), "bascnXxx", "tblXxx");
    assert_eq!(store.app_token(), "bascnXxx");
    assert_eq!(store.table_id(), "tblXxx");
}

#[tokio::test]
#[ignore = "requires LARK_APP_ID and LARK_APP_SECRET"]
async fn integration_append_and_load() {
    use synaptic_core::{MemoryStore, Message};
    let store = LarkBitableMemoryStore::new(
        LarkConfig::new(
            std::env::var("LARK_APP_ID").unwrap(),
            std::env::var("LARK_APP_SECRET").unwrap(),
        ),
        std::env::var("LARK_BITABLE_APP_TOKEN").unwrap(),
        std::env::var("LARK_BITABLE_TABLE_ID").unwrap(),
    );
    let session = "test-session-memory";
    store.clear(session).await.unwrap();
    store
        .append(session, Message::human("Hello"))
        .await
        .unwrap();
    store
        .append(session, Message::ai("Hi there"))
        .await
        .unwrap();
    let msgs = store.load(session).await.unwrap();
    assert_eq!(msgs.len(), 2);
    assert!(msgs[0].content().contains("Hello"));
    assert!(msgs[1].content().contains("Hi there"));
    store.clear(session).await.unwrap();
}
