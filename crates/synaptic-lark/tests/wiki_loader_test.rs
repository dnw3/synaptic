use synaptic_lark::{LarkConfig, LarkWikiLoader};

#[test]
fn builder_defaults() {
    let loader = LarkWikiLoader::new(LarkConfig::new("cli", "secret")).with_space_id("space_xxx");
    assert_eq!(loader.space_id(), "space_xxx");
    assert_eq!(loader.max_depth(), None);
}

#[test]
fn builder_with_depth() {
    let loader = LarkWikiLoader::new(LarkConfig::new("cli", "secret"))
        .with_space_id("space_xxx")
        .with_max_depth(3);
    assert_eq!(loader.max_depth(), Some(3));
}

#[tokio::test]
async fn load_without_space_errors() {
    use synaptic_core::Loader;
    let result = LarkWikiLoader::new(LarkConfig::new("cli", "secret"))
        .load()
        .await;
    assert!(result.is_err());
}
