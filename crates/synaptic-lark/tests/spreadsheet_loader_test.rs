use synaptic_lark::{LarkConfig, LarkSpreadsheetLoader};

#[test]
fn builder() {
    let loader = LarkSpreadsheetLoader::new(LarkConfig::new("cli", "secret"))
        .with_token("shtcnXxx")
        .with_sheet("0")
        .with_content_col(0)
        .with_header_row(true);
    assert_eq!(loader.spreadsheet_token(), "shtcnXxx");
    assert_eq!(loader.sheet_id(), "0");
}

#[tokio::test]
async fn load_without_token_errors() {
    use synaptic_core::Loader;
    let result = LarkSpreadsheetLoader::new(LarkConfig::new("cli", "secret"))
        .load()
        .await;
    assert!(result.is_err());
}
