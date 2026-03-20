# Voice TTS/STT

Voice integration for Synaptic provides text-to-speech (TTS) and speech-to-text (STT) capabilities through pluggable provider traits. The `synaptic-voice` crate defines `TtsProvider` and `SttProvider` traits with built-in implementations for OpenAI and ElevenLabs.

## Setup

Add the `voice` feature to your `Cargo.toml`, along with a provider sub-feature:

```toml
[dependencies]
synaptic = { version = "0.4", features = ["voice"] }
```

Provider sub-features:

| Feature | Provider | Capabilities |
|---------|----------|-------------|
| `openai` | OpenAI | TTS + STT + Streaming TTS |
| `elevenlabs` | ElevenLabs | TTS + Streaming TTS |
| `deepgram` | Deepgram | STT |
| `azure` | Azure Speech | TTS + STT |
| `google` | Google Cloud | STT |
| `all-providers` | All of the above | -- |

## TtsProvider and SttProvider Traits

All voice providers implement one or both of these traits:

```rust,ignore
use synaptic::voice::{TtsProvider, SttProvider, TtsOptions, SttOptions};

// Text-to-Speech: convert text to audio bytes
let audio: Vec<u8> = tts.synthesize("Hello, world!", &TtsOptions::default()).await?;

// Speech-to-Text: transcribe audio bytes to text
let result = stt.transcribe(&audio_bytes, &SttOptions::default()).await?;
println!("Transcribed: {}", result.text);
```

## OpenAI Voice

The `OpenAiVoice` provider supports both TTS and STT through OpenAI's audio APIs. It reads the API key from an environment variable.

### Text-to-Speech

```rust,ignore
use synaptic::voice::openai::OpenAiVoice;
use synaptic::voice::{TtsProvider, TtsOptions, AudioFormat};

let voice = OpenAiVoice::new("OPENAI_API_KEY")?;

// Use the default voice ("alloy") and format (MP3)
let audio = voice.synthesize("Hello from Synaptic!", &TtsOptions::default()).await?;

// Customize voice, format, and speed
let options = TtsOptions {
    voice: "nova".to_string(),
    format: AudioFormat::Wav,
    speed: 1.25,
};
let audio = voice.synthesize("Faster speech in WAV format.", &options).await?;
```

Available voices: `alloy`, `echo`, `fable`, `onyx`, `nova`, `shimmer`.

### Selecting the TTS Model

The default model is `tts-1`. Use `tts-1-hd` for higher quality output:

```rust,ignore
let voice = OpenAiVoice::new("OPENAI_API_KEY")?
    .with_tts_model("tts-1-hd");
```

### Speech-to-Text

```rust,ignore
use synaptic::voice::openai::OpenAiVoice;
use synaptic::voice::{SttProvider, SttOptions, AudioFormat};

let voice = OpenAiVoice::new("OPENAI_API_KEY")?;

let audio_bytes = std::fs::read("recording.mp3")?;

let options = SttOptions {
    language: Some("en".to_string()),
    format: AudioFormat::Mp3,
    prompt: Some("Technical discussion about Rust programming.".to_string()),
};

let result = voice.transcribe(&audio_bytes, &options).await?;
println!("Text: {}", result.text);
if let Some(lang) = &result.language {
    println!("Detected language: {}", lang);
}
if let Some(duration) = result.duration_secs {
    println!("Duration: {:.1}s", duration);
}
```

## ElevenLabs Voice

The `ElevenLabsVoice` provider offers high-quality TTS with configurable voice settings. It supports the `TtsProvider` trait (STT is not available through ElevenLabs).

```rust,ignore
use synaptic::voice::elevenlabs::ElevenLabsVoice;
use synaptic::voice::{TtsProvider, TtsOptions, AudioFormat};

let voice = ElevenLabsVoice::new("ELEVEN_API_KEY")?
    .with_model("eleven_multilingual_v2")
    .with_voice_settings(0.7, 0.8);  // stability, similarity_boost

let options = TtsOptions {
    voice: "your-voice-id".to_string(),
    format: AudioFormat::Mp3,
    speed: 1.0,
};

let audio = voice.synthesize("Bonjour le monde!", &options).await?;
```

### Voice Settings

- `stability` (0.0 -- 1.0, default 0.5) -- Higher values produce more consistent speech; lower values add variation and expressiveness.
- `similarity_boost` (0.0 -- 1.0, default 0.75) -- Higher values make the output sound closer to the original voice sample.

### Listing Available Voices

```rust,ignore
let voices = voice.list_voices().await?;
for name in &voices {
    println!("  {}", name);
}
```

### Output Format Mapping

ElevenLabs uses provider-specific format identifiers internally. The `AudioFormat` enum is mapped as follows:

| AudioFormat | ElevenLabs format |
|-------------|-------------------|
| `Mp3`       | `mp3_44100_128`   |
| `Pcm`       | `pcm_16000`       |
| `Ogg`       | `ogg_vorbis`      |
| `Wav`       | `pcm_16000`       |
| `Flac`      | `pcm_16000`       |

## Deepgram STT

[Deepgram](https://deepgram.com/) provides STT through their Nova-2 model. Enable the `deepgram` feature on `synaptic-voice`.

```rust,ignore
use synaptic::voice::deepgram::DeepgramVoice;
use synaptic::voice::{SttProvider, SttOptions, AudioFormat};

let voice = DeepgramVoice::new("DEEPGRAM_API_KEY")?;

// Optionally select a different model
let voice = DeepgramVoice::new("DEEPGRAM_API_KEY")?
    .with_model("nova-2-general");

let audio = std::fs::read("recording.wav")?;
let result = voice.transcribe(&audio, &SttOptions {
    language: Some("en".to_string()),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;
println!("Transcribed: {}", result.text);
```

## Azure Speech

Azure Cognitive Services Speech supports both TTS and STT. Enable the `azure` feature. Requires `AZURE_SPEECH_KEY` and `AZURE_SPEECH_REGION` environment variables.

```rust,ignore
use synaptic::voice::azure::AzureSpeechVoice;
use synaptic::voice::{TtsProvider, SttProvider, TtsOptions, SttOptions, AudioFormat};

let voice = AzureSpeechVoice::new("AZURE_SPEECH_KEY", "AZURE_SPEECH_REGION")?;

// TTS
let audio = voice.synthesize("Hello from Azure!", &TtsOptions {
    voice: "en-US-JennyNeural".to_string(),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;

// STT
let result = voice.transcribe(&audio, &SttOptions {
    language: Some("en".to_string()),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;
```

## Google Cloud Speech-to-Text

Google Cloud STT uses the Speech v1 REST API. Enable the `google` feature. Requires `GOOGLE_API_KEY` environment variable.

```rust,ignore
use synaptic::voice::google::GoogleSpeechVoice;
use synaptic::voice::{SttProvider, SttOptions, AudioFormat};

let voice = GoogleSpeechVoice::new("GOOGLE_API_KEY")?;

let audio = std::fs::read("recording.wav")?;
let result = voice.transcribe(&audio, &SttOptions {
    language: Some("en".to_string()),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;
println!("Transcribed: {}", result.text);
```

## Streaming TTS

For low-latency audio playback, use `StreamingTtsProvider` which yields audio chunks as they become available instead of buffering the entire response.

```rust,ignore
use futures::StreamExt;
use synaptic::voice::{StreamingTtsProvider, TtsOptions};
use synaptic::voice::openai::OpenAiVoice;

let voice = OpenAiVoice::new("OPENAI_API_KEY")?;
let options = TtsOptions::default();

let mut stream = voice.synthesize_stream("Hello, streaming world!", &options).await?;

while let Some(chunk) = stream.next().await {
    let bytes = chunk?;
    // Write chunk to audio output or file
    println!("Received {} bytes", bytes.len());
}
```

Both `OpenAiVoice` and `ElevenLabsVoice` implement `StreamingTtsProvider`. The trait extends `TtsProvider`, so streaming providers also support the one-shot `synthesize()` method.

### Implementing a Custom Streaming Provider

```rust,ignore
use async_trait::async_trait;
use synaptic::core::SynapticError;
use synaptic::voice::{StreamingTtsProvider, TtsProvider, TtsOptions, TtsStream};

struct MyStreamingTts { /* ... */ }

#[async_trait]
impl TtsProvider for MyStreamingTts {
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<Vec<u8>, SynapticError> {
        // Fallback: collect the stream into a buffer
        use futures::StreamExt;
        let mut stream = self.synthesize_stream(text, options).await?;
        let mut buf = Vec::new();
        while let Some(chunk) = stream.next().await {
            buf.extend_from_slice(&chunk?);
        }
        Ok(buf)
    }
}

#[async_trait]
impl StreamingTtsProvider for MyStreamingTts {
    async fn synthesize_stream(&self, text: &str, options: &TtsOptions) -> Result<TtsStream, SynapticError> {
        // Return a stream of audio chunks from your service
        todo!()
    }
}
```

## Voice Activity Detection (VAD)

The `VadDetector` trait and `EnergyVad` implementation provide voice activity detection -- identifying speech segments in audio data. VAD is always available (no feature flag needed) and has zero external dependencies.

```rust,ignore
use synaptic::voice::{EnergyVad, VadDetector, AudioFormat};

let vad = EnergyVad::default();

// Or customize thresholds
let vad = EnergyVad::default()
    .with_threshold(0.02)      // RMS amplitude threshold
    .with_frame_ms(30)         // Frame duration in milliseconds
    .with_min_speech_ms(100);  // Minimum speech segment duration

let pcm_audio = std::fs::read("recording.pcm")?;
let segments = vad.detect(&pcm_audio, AudioFormat::Pcm).await?;

for seg in &segments {
    println!("Speech: {:.2}s - {:.2}s (probability: {:.2})", seg.start_secs, seg.end_secs, seg.probability);
}
```

**Note:** VAD currently only supports PCM16 audio format. Other formats will return an error.

## Configuration Reference

### TtsOptions

| Field    | Type          | Default   | Description                         |
|----------|---------------|-----------|-------------------------------------|
| `voice`  | `String`      | `"alloy"` | Voice identifier (provider-specific)|
| `format` | `AudioFormat` | `Mp3`     | Output audio format                 |
| `speed`  | `f32`         | `1.0`     | Speech speed multiplier             |

### SttOptions

| Field      | Type             | Default | Description                              |
|------------|------------------|---------|------------------------------------------|
| `language` | `Option<String>` | `None`  | Language hint (ISO 639-1, e.g. `"en"`)   |
| `format`   | `AudioFormat`    | `Mp3`   | Audio format of the input                |
| `prompt`   | `Option<String>` | `None`  | Optional prompt to guide transcription   |

### AudioFormat Variants

| Variant | MIME Type     | Extension |
|---------|---------------|-----------|
| `Mp3`   | `audio/mpeg`  | `mp3`     |
| `Wav`   | `audio/wav`   | `wav`     |
| `Ogg`   | `audio/ogg`   | `ogg`     |
| `Flac`  | `audio/flac`  | `flac`    |
| `Pcm`   | `audio/pcm`   | `pcm`     |

## Custom Provider

Implement `TtsProvider` and/or `SttProvider` to add your own voice backend:

```rust,ignore
use async_trait::async_trait;
use synaptic::core::SynapticError;
use synaptic::voice::{TtsProvider, TtsOptions, AudioFormat};

struct MyTtsProvider { /* ... */ }

#[async_trait]
impl TtsProvider for MyTtsProvider {
    async fn synthesize(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> Result<Vec<u8>, SynapticError> {
        // Call your TTS service here
        let audio_bytes = my_tts_service::synthesize(text, &options.voice).await
            .map_err(|e| SynapticError::Model(format!("TTS failed: {}", e)))?;
        Ok(audio_bytes)
    }

    async fn list_voices(&self) -> Result<Vec<String>, SynapticError> {
        Ok(vec!["default".to_string(), "narrator".to_string()])
    }
}
```
