use synaptic_lark::{LarkConfig, LarkDriveLoader};

#[test]
fn builder() {
    let loader =
        LarkDriveLoader::new(LarkConfig::new("cli", "secret")).with_folder_token("fldcnXxx");
    assert_eq!(loader.folder_token(), "fldcnXxx");
}

#[tokio::test]
async fn load_without_folder_errors() {
    use synaptic_core::Loader;
    let result = LarkDriveLoader::new(LarkConfig::new("cli", "secret"))
        .load()
        .await;
    assert!(result.is_err());
}
