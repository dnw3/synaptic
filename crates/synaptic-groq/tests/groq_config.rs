use synaptic_groq::{GroqConfig, GroqModel};
use synaptic_openai::OpenAiConfig;

#[test]
fn test_model_enum_as_str() {
    assert_eq!(
        GroqModel::Llama3_3_70bVersatile.as_str(),
        "llama-3.3-70b-versatile"
    );
    assert_eq!(
        GroqModel::Llama3_1_8bInstant.as_str(),
        "llama-3.1-8b-instant"
    );
    assert_eq!(GroqModel::Mixtral8x7b32768.as_str(), "mixtral-8x7b-32768");
    assert_eq!(
        GroqModel::Custom("my-model".to_string()).as_str(),
        "my-model"
    );
}

#[test]
fn test_config_builder() {
    let config = GroqConfig::new("key", GroqModel::Llama3_3_70bVersatile)
        .with_temperature(0.7)
        .with_max_tokens(1024)
        .with_seed(42);
    assert_eq!(config.temperature, Some(0.7));
    assert_eq!(config.max_tokens, Some(1024));
    assert_eq!(config.seed, Some(42));
}

#[test]
fn test_config_into_openai() {
    let config = GroqConfig::new("gsk-key", GroqModel::Llama3_3_70bVersatile).with_temperature(0.5);
    let oa: OpenAiConfig = config.into();
    assert_eq!(oa.base_url, "https://api.groq.com/openai/v1");
    assert_eq!(oa.temperature, Some(0.5));
}
