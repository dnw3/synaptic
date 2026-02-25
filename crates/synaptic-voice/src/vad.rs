//! Voice Activity Detection (VAD).
//!
//! Provides a [`VadDetector`] trait and an energy-based implementation
//! that analyzes PCM audio amplitude to detect speech segments.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;

use crate::AudioFormat;

/// A detected speech segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadSegment {
    /// Start time in seconds.
    pub start_secs: f64,
    /// End time in seconds.
    pub end_secs: f64,
    /// Speech probability (0.0–1.0).
    pub probability: f32,
}

/// Voice Activity Detection trait.
#[async_trait]
pub trait VadDetector: Send + Sync {
    /// Detect speech segments in audio data.
    async fn detect(
        &self,
        audio: &[u8],
        format: AudioFormat,
    ) -> Result<Vec<VadSegment>, SynapticError>;
}

/// Energy-based Voice Activity Detector.
///
/// Analyzes RMS amplitude of PCM frames against a threshold.
/// No external dependencies — works on raw PCM16 audio.
pub struct EnergyVad {
    /// RMS threshold for speech detection (default: 0.02).
    pub threshold: f32,
    /// Frame duration in milliseconds (default: 30).
    pub frame_ms: u32,
    /// Minimum speech duration in milliseconds (default: 250).
    pub min_speech_ms: u32,
    /// Sample rate in Hz (default: 16000).
    pub sample_rate: u32,
}

impl Default for EnergyVad {
    fn default() -> Self {
        Self {
            threshold: 0.02,
            frame_ms: 30,
            min_speech_ms: 250,
            sample_rate: 16000,
        }
    }
}

impl EnergyVad {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn with_frame_ms(mut self, frame_ms: u32) -> Self {
        self.frame_ms = frame_ms;
        self
    }

    pub fn with_min_speech_ms(mut self, min_speech_ms: u32) -> Self {
        self.min_speech_ms = min_speech_ms;
        self
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// Compute RMS of a PCM16 LE frame.
    fn rms(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
        ((sum / samples.len() as f64).sqrt() / 32768.0) as f32
    }

    /// Parse raw bytes as PCM16 little-endian samples.
    fn bytes_to_samples(data: &[u8]) -> Vec<i16> {
        data.chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }
}

#[async_trait]
impl VadDetector for EnergyVad {
    async fn detect(
        &self,
        audio: &[u8],
        format: AudioFormat,
    ) -> Result<Vec<VadSegment>, SynapticError> {
        if format != AudioFormat::Pcm && format != AudioFormat::Wav {
            return Err(SynapticError::Config(
                "EnergyVad requires PCM or WAV audio input".to_string(),
            ));
        }

        // Skip WAV header if present (44 bytes)
        let pcm_data = if format == AudioFormat::Wav && audio.len() > 44 {
            &audio[44..]
        } else {
            audio
        };

        let samples = Self::bytes_to_samples(pcm_data);
        let frame_samples = (self.sample_rate * self.frame_ms / 1000) as usize;
        if frame_samples == 0 {
            return Ok(vec![]);
        }
        let min_frames = (self.min_speech_ms / self.frame_ms) as usize;
        let frame_duration = self.frame_ms as f64 / 1000.0;

        let mut segments = Vec::new();
        let mut speech_start: Option<usize> = None;
        let mut speech_frames = 0usize;

        for (i, frame) in samples.chunks(frame_samples).enumerate() {
            let rms = Self::rms(frame);
            let is_speech = rms >= self.threshold;

            if is_speech {
                if speech_start.is_none() {
                    speech_start = Some(i);
                }
                speech_frames += 1;
            } else if let Some(start) = speech_start {
                if speech_frames >= min_frames {
                    segments.push(VadSegment {
                        start_secs: start as f64 * frame_duration,
                        end_secs: i as f64 * frame_duration,
                        probability: 1.0,
                    });
                }
                speech_start = None;
                speech_frames = 0;
            }
        }

        // Handle segment that extends to end of audio
        if let Some(start) = speech_start {
            if speech_frames >= min_frames {
                let total_frames = samples.len().div_ceil(frame_samples);
                segments.push(VadSegment {
                    start_secs: start as f64 * frame_duration,
                    end_secs: total_frames as f64 * frame_duration,
                    probability: 1.0,
                });
            }
        }

        Ok(segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_silence() {
        let samples = vec![0i16; 480];
        assert_eq!(EnergyVad::rms(&samples), 0.0);
    }

    #[test]
    fn rms_loud() {
        let samples = vec![16384i16; 480];
        let rms = EnergyVad::rms(&samples);
        assert!(rms > 0.4 && rms < 0.6, "rms = {}", rms);
    }

    #[test]
    fn bytes_to_samples_roundtrip() {
        let original: Vec<i16> = vec![0, 1000, -1000, 32767, -32768];
        let bytes: Vec<u8> = original.iter().flat_map(|s| s.to_le_bytes()).collect();
        let result = EnergyVad::bytes_to_samples(&bytes);
        assert_eq!(result, original);
    }

    #[tokio::test]
    async fn detect_silence() {
        let vad = EnergyVad::new();
        let silence = vec![0u8; 32000]; // 1s at 16kHz
        let segments = vad.detect(&silence, AudioFormat::Pcm).await.unwrap();
        assert!(segments.is_empty());
    }

    #[tokio::test]
    async fn detect_speech() {
        let vad = EnergyVad::new().with_threshold(0.01).with_min_speech_ms(30);
        let mut audio = Vec::with_capacity(32000);
        for _ in 0..16000 {
            audio.extend_from_slice(&5000i16.to_le_bytes());
        }
        let segments = vad.detect(&audio, AudioFormat::Pcm).await.unwrap();
        assert!(!segments.is_empty());
    }

    #[tokio::test]
    async fn rejects_non_pcm() {
        let vad = EnergyVad::new();
        let result = vad.detect(&[0; 100], AudioFormat::Mp3).await;
        assert!(result.is_err());
    }

    #[test]
    fn default_config() {
        let vad = EnergyVad::default();
        assert!((vad.threshold - 0.02).abs() < f32::EPSILON);
        assert_eq!(vad.frame_ms, 30);
        assert_eq!(vad.min_speech_ms, 250);
        assert_eq!(vad.sample_rate, 16000);
    }
}
