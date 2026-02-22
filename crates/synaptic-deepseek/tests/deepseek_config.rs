use synaptic_deepseek::{DeepSeekConfig, DeepSeekModel};
use synaptic_openai::OpenAiConfig;

#[test]
fn test_model_enum_as_str() {
    assert_eq!(DeepSeekModel::DeepSeekChat.as_str(), "deepseek-chat");
    assert_eq!(
        DeepSeekModel::DeepSeekReasoner.as_str(),
        "deepseek-reasoner"
    );
    assert_eq!(DeepSeekModel::DeepSeekCoderV2.as_str(), "deepseek-coder-v2");
    assert_eq!(DeepSeekModel::Custom("x".to_string()).as_str(), "x");
}

#[test]
fn test_config_into_openai() {
    let config = DeepSeekConfig::new("sk-key", DeepSeekModel::DeepSeekChat).with_max_tokens(2048);
    let oa: OpenAiConfig = config.into();
    assert_eq!(oa.base_url, "https://api.deepseek.com/v1");
    assert_eq!(oa.max_tokens, Some(2048));
}
