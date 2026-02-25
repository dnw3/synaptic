//! OpenAI TTS (tts-1) and STT (whisper-1) providers.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::multipart;
use serde_json::json;
use synaptic_core::SynapticError;

use crate::{
    AudioFormat, StreamingTtsProvider, SttOptions, SttProvider, Transcription, TtsOptions,
    TtsProvider, TtsStream,
};

/// OpenAI TTS/STT provider.
pub struct OpenAiVoice {
    client: reqwest::Client,
    api_key: String,
    tts_model: String,
    stt_model: String,
}

impl OpenAiVoice {
    /// Create a new OpenAI voice provider.
    ///
    /// Reads the API key from the specified environment variable.
    pub fn new(api_key_env: &str) -> Result<Self, SynapticError> {
        let api_key = std::env::var(api_key_env)
            .map_err(|_| SynapticError::Config(format!("env var '{}' not set", api_key_env)))?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            tts_model: "tts-1".to_string(),
            stt_model: "whisper-1".to_string(),
        })
    }

    /// Set the TTS model (e.g. "tts-1", "tts-1-hd").
    pub fn with_tts_model(mut self, model: &str) -> Self {
        self.tts_model = model.to_string();
        self
    }

    /// Set the STT model (e.g. "whisper-1").
    pub fn with_stt_model(mut self, model: &str) -> Self {
        self.stt_model = model.to_string();
        self
    }

    fn format_str(format: &AudioFormat) -> &'static str {
        match format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "opus",
            AudioFormat::Flac => "flac",
            AudioFormat::Pcm => "pcm",
        }
    }
}

#[async_trait]
impl TtsProvider for OpenAiVoice {
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<Vec<u8>, SynapticError> {
        let body = json!({
            "model": self.tts_model,
            "input": text,
            "voice": options.voice,
            "response_format": Self::format_str(&options.format),
            "speed": options.speed,
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("TTS request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "TTS error {}: {}",
                status, text
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| SynapticError::Model(format!("TTS read error: {}", e)))
    }

    async fn list_voices(&self) -> Result<Vec<String>, SynapticError> {
        Ok(vec![
            "alloy".to_string(),
            "echo".to_string(),
            "fable".to_string(),
            "onyx".to_string(),
            "nova".to_string(),
            "shimmer".to_string(),
        ])
    }
}

#[async_trait]
impl SttProvider for OpenAiVoice {
    async fn transcribe(
        &self,
        audio: &[u8],
        options: &SttOptions,
    ) -> Result<Transcription, SynapticError> {
        let filename = format!("audio.{}", options.format.extension());
        let mime = options.format.mime_type().to_string();

        let file_part = multipart::Part::bytes(audio.to_vec())
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| SynapticError::Model(format!("multipart error: {}", e)))?;

        let mut form = multipart::Form::new()
            .text("model", self.stt_model.clone())
            .part("file", file_part);

        if let Some(ref lang) = options.language {
            form = form.text("language", lang.clone());
        }
        if let Some(ref prompt) = options.prompt {
            form = form.text("prompt", prompt.clone());
        }

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("STT request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "STT error {}: {}",
                status, text
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Model(format!("STT parse error: {}", e)))?;

        Ok(Transcription {
            text: body["text"].as_str().unwrap_or("").to_string(),
            language: body["language"].as_str().map(|s| s.to_string()),
            duration_secs: body["duration"].as_f64(),
        })
    }
}

#[async_trait]
impl StreamingTtsProvider for OpenAiVoice {
    async fn synthesize_stream(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> Result<TtsStream, SynapticError> {
        let body = json!({
            "model": self.tts_model,
            "input": text,
            "voice": options.voice,
            "response_format": Self::format_str(&options.format),
            "speed": options.speed,
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("TTS stream request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "TTS stream error {}: {}",
                status, text
            )));
        }

        let stream = resp.bytes_stream().map(|result| {
            result.map_err(|e| SynapticError::Model(format!("TTS stream read error: {}", e)))
        });

        Ok(Box::pin(stream))
    }
}
