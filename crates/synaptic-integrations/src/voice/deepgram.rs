//! Deepgram STT provider (Nova-2).

use async_trait::async_trait;
use synaptic_core::SynapticError;

use super::{SttOptions, SttProvider, Transcription};

/// Deepgram speech-to-text provider.
pub struct DeepgramVoice {
    client: reqwest::Client,
    api_key: String,
    /// Model name (default: "nova-2").
    model: String,
}

impl DeepgramVoice {
    /// Create a new Deepgram STT provider.
    ///
    /// Reads the API key from the environment variable named `api_key_env`.
    pub fn new(api_key_env: &str) -> Result<Self, SynapticError> {
        let api_key = std::env::var(api_key_env)
            .map_err(|_| SynapticError::Config(format!("env var '{}' not set", api_key_env)))?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: "nova-2".to_string(),
        })
    }

    /// Set the model (e.g. "nova-2", "nova-2-general", "whisper-large").
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }
}

#[async_trait]
impl SttProvider for DeepgramVoice {
    async fn transcribe(
        &self,
        audio: &[u8],
        options: &SttOptions,
    ) -> Result<Transcription, SynapticError> {
        let mut url = format!("https://api.deepgram.com/v1/listen?model={}", self.model);
        if let Some(ref lang) = options.language {
            url.push_str(&format!("&language={}", lang));
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", options.format.mime_type())
            .body(audio.to_vec())
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("Deepgram STT request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "Deepgram STT error {}: {}",
                status, text
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Model(format!("Deepgram STT parse error: {}", e)))?;

        let transcript = body["results"]["channels"][0]["alternatives"][0]["transcript"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let duration_secs = body["metadata"]["duration"].as_f64();

        let language = body["results"]["channels"][0]["detected_language"]
            .as_str()
            .map(|s| s.to_string());

        Ok(Transcription {
            text: transcript,
            language,
            duration_secs,
        })
    }
}
