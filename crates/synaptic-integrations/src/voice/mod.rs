//! Voice TTS/STT providers for the Synaptic AI agent framework.
//!
//! Provides [`TtsProvider`] and [`SttProvider`] traits for text-to-speech
//! and speech-to-text conversion, with optional provider implementations.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;

#[cfg(feature = "voice-openai")]
pub mod openai;
#[cfg(feature = "voice-openai")]
pub use openai::OpenAiVoice;

#[cfg(feature = "voice-elevenlabs")]
pub mod elevenlabs;
#[cfg(feature = "voice-elevenlabs")]
pub use elevenlabs::ElevenLabsVoice;

#[cfg(feature = "voice-deepgram")]
pub mod deepgram;

#[cfg(feature = "voice-azure")]
pub mod azure;

#[cfg(feature = "voice-google")]
pub mod google;

pub mod vad;
pub use vad::{EnergyVad, VadDetector, VadSegment};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Audio format for TTS output or STT input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
    Flac,
    Pcm,
}

impl AudioFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Ogg => "audio/ogg",
            Self::Flac => "audio/flac",
            Self::Pcm => "audio/pcm",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Flac => "flac",
            Self::Pcm => "pcm",
        }
    }
}

/// Options for TTS synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsOptions {
    /// Voice identifier (provider-specific).
    pub voice: String,
    /// Output audio format.
    #[serde(default = "default_format")]
    pub format: AudioFormat,
    /// Speech speed multiplier (1.0 = normal).
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_format() -> AudioFormat {
    AudioFormat::Mp3
}

fn default_speed() -> f32 {
    1.0
}

impl Default for TtsOptions {
    fn default() -> Self {
        Self {
            voice: "alloy".to_string(),
            format: default_format(),
            speed: default_speed(),
        }
    }
}

/// Options for STT transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttOptions {
    /// Language hint (ISO 639-1, e.g. "en").
    pub language: Option<String>,
    /// Audio format of the input.
    #[serde(default = "default_format")]
    pub format: AudioFormat,
    /// Optional prompt to guide transcription.
    pub prompt: Option<String>,
}

impl Default for SttOptions {
    fn default() -> Self {
        Self {
            language: None,
            format: default_format(),
            prompt: None,
        }
    }
}

/// Result of a transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    /// The transcribed text.
    pub text: String,
    /// Detected language (if available).
    pub language: Option<String>,
    /// Duration of the audio in seconds (if available).
    pub duration_secs: Option<f64>,
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Text-to-Speech provider.
#[async_trait]
pub trait TtsProvider: Send + Sync {
    /// Convert text to speech audio bytes.
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<Vec<u8>, SynapticError>;

    /// List available voices.
    async fn list_voices(&self) -> Result<Vec<String>, SynapticError> {
        Ok(vec![])
    }
}

/// Speech-to-Text provider.
#[async_trait]
pub trait SttProvider: Send + Sync {
    /// Transcribe audio bytes to text.
    async fn transcribe(
        &self,
        audio: &[u8],
        options: &SttOptions,
    ) -> Result<Transcription, SynapticError>;
}

/// A stream of audio chunks for streaming TTS.
pub type TtsStream = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, SynapticError>> + Send>>;

/// Streaming Text-to-Speech provider.
///
/// Extends [`TtsProvider`] with real-time audio streaming. Providers yield chunks
/// as they become available, enabling low-latency playback.
#[async_trait]
pub trait StreamingTtsProvider: TtsProvider {
    /// Synthesize text to a stream of audio chunks.
    async fn synthesize_stream(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> Result<TtsStream, SynapticError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_mime() {
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
        assert_eq!(AudioFormat::Wav.mime_type(), "audio/wav");
        assert_eq!(AudioFormat::Ogg.mime_type(), "audio/ogg");
    }

    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Flac.extension(), "flac");
    }

    #[test]
    fn test_tts_options_default() {
        let opts = TtsOptions::default();
        assert_eq!(opts.voice, "alloy");
        assert_eq!(opts.format, AudioFormat::Mp3);
        assert!((opts.speed - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stt_options_default() {
        let opts = SttOptions::default();
        assert!(opts.language.is_none());
        assert_eq!(opts.format, AudioFormat::Mp3);
    }

    #[test]
    fn tts_stream_type_exists() {
        // Type-level assertion — TtsStream exists and is Send
        fn _assert_send<T: Send>() {}
        _assert_send::<TtsStream>();
        let _ = std::any::type_name::<TtsStream>();
    }
}
