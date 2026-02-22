use synaptic_lark::{LarkBitableLoader, LarkConfig};

#[test]
fn builder_defaults() {
    let loader = LarkBitableLoader::new(LarkConfig::new("cli", "secret"))
        .with_app("bascnXxx")
        .with_table("tblXxx");
    assert_eq!(loader.app_token(), "bascnXxx");
    assert_eq!(loader.table_id(), "tblXxx");
    assert!(loader.view_id().is_none());
    assert_eq!(loader.content_field(), None);
}

#[test]
fn builder_with_view_and_content_field() {
    let loader = LarkBitableLoader::new(LarkConfig::new("cli", "secret"))
        .with_app("bascnXxx")
        .with_table("tblXxx")
        .with_view("vewXxx")
        .with_content_field("Description");
    assert_eq!(loader.view_id(), Some("vewXxx"));
    assert_eq!(loader.content_field(), Some("Description"));
}

#[tokio::test]
async fn load_returns_error_without_app() {
    use synaptic_core::Loader;
    let loader = LarkBitableLoader::new(LarkConfig::new("cli", "secret"));
    let result = loader.load().await;
    assert!(result.is_err());
}
