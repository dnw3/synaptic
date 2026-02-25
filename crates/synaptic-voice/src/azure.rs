//! Azure Speech Services TTS and STT provider.

use async_trait::async_trait;
use synaptic_core::SynapticError;

use crate::{AudioFormat, SttOptions, SttProvider, Transcription, TtsOptions, TtsProvider};

/// Azure Speech Services provider (STT + TTS).
pub struct AzureSpeechVoice {
    client: reqwest::Client,
    api_key: String,
    region: String,
}

impl AzureSpeechVoice {
    /// Create a new Azure Speech provider.
    ///
    /// Reads the API key from the environment variable named `api_key_env`.
    /// `region` is the Azure region (e.g. "eastus", "westeurope").
    pub fn new(api_key_env: &str, region: &str) -> Result<Self, SynapticError> {
        let api_key = std::env::var(api_key_env)
            .map_err(|_| SynapticError::Config(format!("env var '{}' not set", api_key_env)))?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            region: region.to_string(),
        })
    }
}

/// Map `AudioFormat` to the Azure `Content-Type` header value for STT.
fn azure_stt_content_type(format: &AudioFormat) -> &'static str {
    match format {
        AudioFormat::Wav => "audio/wav",
        AudioFormat::Mp3 => "audio/mpeg",
        AudioFormat::Ogg => "audio/ogg",
        AudioFormat::Flac => "audio/flac",
        AudioFormat::Pcm => "audio/wav", // PCM sent as raw WAV
    }
}

/// Map `AudioFormat` to the Azure `X-Microsoft-OutputFormat` header value for TTS.
fn azure_tts_output_format(format: &AudioFormat) -> &'static str {
    match format {
        AudioFormat::Mp3 => "audio-16khz-128kbitrate-mono-mp3",
        AudioFormat::Wav => "riff-16khz-16bit-mono-pcm",
        AudioFormat::Ogg => "ogg-16khz-16bit-mono-opus",
        AudioFormat::Flac => "audio-16khz-128kbitrate-mono-mp3", // no native FLAC; fall back to MP3
        AudioFormat::Pcm => "raw-16khz-16bit-mono-pcm",
    }
}

#[async_trait]
impl SttProvider for AzureSpeechVoice {
    async fn transcribe(
        &self,
        audio: &[u8],
        options: &SttOptions,
    ) -> Result<Transcription, SynapticError> {
        let lang = options.language.as_deref().unwrap_or("en-US");

        let url = format!(
            "https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1?language={}",
            self.region, lang
        );

        let content_type = azure_stt_content_type(&options.format);

        let resp = self
            .client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &self.api_key)
            .header("Content-Type", content_type)
            .body(audio.to_vec())
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("Azure STT request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "Azure STT error {}: {}",
                status, text
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SynapticError::Model(format!("Azure STT parse error: {}", e)))?;

        let text = body["DisplayText"].as_str().unwrap_or("").to_string();

        // Azure returns Duration in 100-nanosecond ticks.
        let duration_secs = body["Duration"]
            .as_u64()
            .map(|ticks| ticks as f64 / 10_000_000.0);

        Ok(Transcription {
            text,
            language: Some(lang.to_string()),
            duration_secs,
        })
    }
}

#[async_trait]
impl TtsProvider for AzureSpeechVoice {
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<Vec<u8>, SynapticError> {
        let url = format!(
            "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
            self.region
        );

        let output_format = azure_tts_output_format(&options.format);

        // Build SSML body.
        let ssml = format!(
            r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US"><voice name="{}">{}</voice></speak>"#,
            options.voice,
            quick_xml_escape(text),
        );

        let resp = self
            .client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &self.api_key)
            .header("Content-Type", "application/ssml+xml")
            .header("X-Microsoft-OutputFormat", output_format)
            .body(ssml)
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("Azure TTS request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Model(format!(
                "Azure TTS error {}: {}",
                status, text
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| SynapticError::Model(format!("Azure TTS read error: {}", e)))
    }
}

/// Minimal XML escaping for SSML text content.
fn quick_xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
