//! ElevenLabs TTS provider.

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::json;
use synaptic_core::SynapticError;

use super::{AudioFormat, StreamingTtsProvider, TtsOptions, TtsProvider, TtsStream};

/// ElevenLabs TTS provider with configurable model, format, and voice settings.
pub struct ElevenLabsVoice {
    client: reqwest::Client,
    api_key: String,
    /// Model ID (default: "eleven_multilingual_v2").
    model_id: String,
    /// Voice stability (0.0–1.0, default: 0.5).
    stability: f32,
    /// Similarity boost (0.0–1.0, default: 0.75).
    similarity_boost: f32,
}

impl ElevenLabsVoice {
    /// Create a new ElevenLabs voice provider.
    ///
    /// Reads the API key from the environment variable named `api_key_env`.
    pub fn new(api_key_env: &str) -> Result<Self, SynapticError> {
        let api_key = std::env::var(api_key_env)
            .map_err(|_| SynapticError::Config(format!("env var '{}' not set", api_key_env)))?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model_id: "eleven_multilingual_v2".to_string(),
            stability: 0.5,
            similarity_boost: 0.75,
        })
    }

    /// Set the model ID (e.g., "eleven_multilingual_v2", "eleven_turbo_v2_5").
    pub fn with_model(mut self, model_id: &str) -> Self {
        self.model_id = model_id.to_string();
        self
    }

    /// Set voice settings for stability and similarity boost.
    ///
    /// - `stability`: 0.0 (more variable) to 1.0 (more stable). Default: 0.5.
    /// - `similarity_boost`: 0.0 (less similar) to 1.0 (more similar). Default: 0.75.
    pub fn with_voice_settings(mut self, stability: f32, similarity_boost: f32) -> Self {
        self.stability = stability;
        self.similarity_boost = similarity_boost;
        self
    }
}

/// Map `AudioFormat` to ElevenLabs output_format query parameter.
fn elevenlabs_output_format(format: AudioFormat) -> &'static str {
    match format {
        AudioFormat::Mp3 => "mp3_44100_128",
        AudioFormat::Pcm => "pcm_16000",
        AudioFormat::Ogg => "ogg_vorbis",
        // WAV/FLAC: use PCM as closest match
        AudioFormat::Wav | AudioFormat::Flac => "pcm_16000",
    }
}

#[async_trait]
impl TtsProvider for ElevenLabsVoice {
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<Vec<u8>, SynapticError> {
        let body = json!({
            "text": text,
            "model_id": self.model_id,
            "voice_settings": {
                "stability": self.stability,
                "similarity_boost": self.similarity_boost,
            },
        });

        let output_format = elevenlabs_output_format(options.format);
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format={}",
            options.voice, output_format
        );

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("ElevenLabs TTS request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "ElevenLabs TTS error {}: {}",
                status, text
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| SynapticError::Model(format!("ElevenLabs TTS read error: {}", e)))
    }

    async fn list_voices(&self) -> Result<Vec<String>, SynapticError> {
        let resp = self
            .client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("ElevenLabs list voices failed: {}", e)))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Model(format!("ElevenLabs parse error: {}", e)))?;

        let voices = body["voices"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(voices)
    }
}

#[async_trait]
impl StreamingTtsProvider for ElevenLabsVoice {
    async fn synthesize_stream(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> Result<TtsStream, SynapticError> {
        let body = json!({
            "text": text,
            "model_id": self.model_id,
            "voice_settings": {
                "stability": self.stability,
                "similarity_boost": self.similarity_boost,
            },
        });

        let output_format = elevenlabs_output_format(options.format);
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format={}",
            options.voice, output_format
        );

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("ElevenLabs TTS stream failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "ElevenLabs TTS stream error {}: {}",
                status, text
            )));
        }

        let stream = resp.bytes_stream().map(|result| {
            result.map_err(|e| SynapticError::Model(format!("ElevenLabs stream read error: {}", e)))
        });

        Ok(Box::pin(stream))
    }
}
