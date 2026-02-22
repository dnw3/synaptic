use synaptic_mistral::{MistralConfig, MistralModel};
use synaptic_openai::OpenAiConfig;

#[test]
fn test_model_enum_as_str() {
    assert_eq!(
        MistralModel::MistralLargeLatest.as_str(),
        "mistral-large-latest"
    );
    assert_eq!(
        MistralModel::MistralSmallLatest.as_str(),
        "mistral-small-latest"
    );
    assert_eq!(MistralModel::OpenMistralNemo.as_str(), "open-mistral-nemo");
    assert_eq!(MistralModel::CodestralLatest.as_str(), "codestral-latest");
    assert_eq!(MistralModel::Custom("x".to_string()).as_str(), "x");
}

#[test]
fn test_config_into_openai() {
    let config = MistralConfig::new("key", MistralModel::MistralLargeLatest).with_temperature(0.3);
    let oa: OpenAiConfig = config.into();
    assert_eq!(oa.base_url, "https://api.mistral.ai/v1");
    assert_eq!(oa.temperature, Some(0.3));
}
