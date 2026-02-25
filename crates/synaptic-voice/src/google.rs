//! Google Cloud Speech-to-Text provider.

use async_trait::async_trait;
use base64::Engine;
use serde_json::json;
use synaptic_core::SynapticError;

use crate::{AudioFormat, SttOptions, SttProvider, Transcription};

/// Google Cloud Speech-to-Text provider.
pub struct GoogleSpeechVoice {
    client: reqwest::Client,
    api_key: String,
}

impl GoogleSpeechVoice {
    /// Create a new Google Cloud STT provider.
    ///
    /// Reads the API key from the environment variable named `api_key_env`.
    pub fn new(api_key_env: &str) -> Result<Self, SynapticError> {
        let api_key = std::env::var(api_key_env)
            .map_err(|_| SynapticError::Config(format!("env var '{}' not set", api_key_env)))?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
        })
    }
}

/// Map `AudioFormat` to the Google Cloud encoding string.
fn google_encoding(format: &AudioFormat) -> &'static str {
    match format {
        AudioFormat::Wav => "LINEAR16",
        AudioFormat::Flac => "FLAC",
        AudioFormat::Mp3 => "MP3",
        AudioFormat::Ogg => "OGG_OPUS",
        AudioFormat::Pcm => "LINEAR16",
    }
}

#[async_trait]
impl SttProvider for GoogleSpeechVoice {
    async fn transcribe(
        &self,
        audio: &[u8],
        options: &SttOptions,
    ) -> Result<Transcription, SynapticError> {
        let url = format!(
            "https://speech.googleapis.com/v1/speech:recognize?key={}",
            self.api_key
        );

        let audio_content = base64::engine::general_purpose::STANDARD.encode(audio);
        let encoding = google_encoding(&options.format);

        let mut config = json!({
            "encoding": encoding,
        });

        if let Some(ref lang) = options.language {
            config["languageCode"] = json!(lang);
        } else {
            config["languageCode"] = json!("en-US");
        }

        let body = json!({
            "config": config,
            "audio": {
                "content": audio_content,
            },
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("Google STT request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "Google STT error {}: {}",
                status, text
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Model(format!("Google STT parse error: {}", e)))?;

        let transcript = body["results"][0]["alternatives"][0]["transcript"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let language = body["results"][0]["languageCode"]
            .as_str()
            .map(|s| s.to_string());

        Ok(Transcription {
            text: transcript,
            language,
            duration_secs: None,
        })
    }
}
