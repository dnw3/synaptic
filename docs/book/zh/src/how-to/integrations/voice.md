# 语音 TTS/STT

语音集成提供文本转语音 (TTS) 和语音转文本 (STT) 功能，通过可插拔的 provider trait 实现。

## 简介

`synaptic-voice` crate 定义了两个核心 trait：

- **`TtsProvider`** -- 将文本合成为音频字节。
- **`SttProvider`** -- 将音频字节转录为文本。

框架内置了 OpenAI 和 ElevenLabs 两个 provider 实现，同时支持通过实现 trait 接入自定义语音服务。

## 安装

在 `Cargo.toml` 中添加 `voice` feature，并选择所需的子 feature：

```toml
[dependencies]
synaptic = { version = "0.3", features = ["voice"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Provider 子 feature：

| Feature | 提供商 | 功能 |
|---------|--------|------|
| `openai` | OpenAI | TTS + STT + 流式 TTS |
| `elevenlabs` | ElevenLabs | TTS + 流式 TTS |
| `deepgram` | Deepgram | STT |
| `azure` | Azure Speech | TTS + STT |
| `google` | Google Cloud | STT |
| `all-providers` | 以上全部 | -- |

## OpenAI 语音

OpenAI provider 同时支持 TTS（`tts-1`、`tts-1-hd` 模型）和 STT（`whisper-1` 模型）。

### 文本转语音 (TTS)

```rust,ignore
use synaptic::voice::{OpenAiVoice, TtsProvider, TtsOptions, AudioFormat};

let voice = OpenAiVoice::new("OPENAI_API_KEY")?
    .with_tts_model("tts-1-hd");

let options = TtsOptions {
    voice: "nova".to_string(),
    format: AudioFormat::Mp3,
    speed: 1.0,
};

let audio_bytes = voice.synthesize("你好，世界！", &options).await?;
std::fs::write("output.mp3", &audio_bytes)?;
```

OpenAI 提供以下预置语音名称：

| 语音 | 风格描述 |
|------|----------|
| `alloy` | 中性，平衡 |
| `echo` | 温暖，自信 |
| `fable` | 富有表现力 |
| `onyx` | 深沉，权威 |
| `nova` | 友好，活泼 |
| `shimmer` | 清晰，明亮 |

### 语音转文本 (STT)

```rust,ignore
use synaptic::voice::{OpenAiVoice, SttProvider, SttOptions, AudioFormat};

let voice = OpenAiVoice::new("OPENAI_API_KEY")?;

let audio_data = std::fs::read("recording.mp3")?;

let options = SttOptions {
    language: Some("zh".to_string()),
    format: AudioFormat::Mp3,
    prompt: Some("这是一段关于 Rust 编程的讨论。".to_string()),
};

let transcription = voice.transcribe(&audio_data, &options).await?;
println!("转录文本: {}", transcription.text);
println!("检测语言: {:?}", transcription.language);
println!("音频时长: {:?} 秒", transcription.duration_secs);
```

## ElevenLabs 语音

ElevenLabs provider 仅支持 TTS，提供高质量的多语言语音合成。

```rust,ignore
use synaptic::voice::{ElevenLabsVoice, TtsProvider, TtsOptions, AudioFormat};

let voice = ElevenLabsVoice::new("ELEVENLABS_API_KEY")?
    .with_model("eleven_multilingual_v2")
    .with_voice_settings(0.7, 0.8);  // stability, similarity_boost
```

**参数说明：**

- `with_model` -- 模型 ID，默认为 `"eleven_multilingual_v2"`。其他可选模型包括 `"eleven_turbo_v2_5"`。
- `with_voice_settings(stability, similarity_boost)` -- 调整语音特征：
  - `stability`：0.0（更有变化）到 1.0（更稳定），默认 0.5。
  - `similarity_boost`：0.0（较低相似度）到 1.0（较高相似度），默认 0.75。

**输出格式映射：**

ElevenLabs API 使用特定的格式字符串，`AudioFormat` 枚举会自动映射：

| AudioFormat | ElevenLabs 格式 |
|-------------|-----------------|
| `Mp3` | `mp3_44100_128` |
| `Pcm` | `pcm_16000` |
| `Ogg` | `ogg_vorbis` |
| `Wav` | `pcm_16000`（最接近匹配） |
| `Flac` | `pcm_16000`（最接近匹配） |

**合成音频：**

```rust,ignore
let options = TtsOptions {
    voice: "your-voice-id".to_string(),
    format: AudioFormat::Mp3,
    speed: 1.0,
};

let audio = voice.synthesize("Hello from ElevenLabs!", &options).await?;
```

**列出可用语音：**

```rust,ignore
let voices = voice.list_voices().await?;
for name in &voices {
    println!("可用语音: {}", name);
}
```

## Deepgram 语音识别

[Deepgram](https://deepgram.com/) 通过 Nova-2 模型提供语音识别。在 `synaptic-voice` 上启用 `deepgram` feature。

```rust,ignore
use synaptic::voice::deepgram::DeepgramVoice;
use synaptic::voice::{SttProvider, SttOptions, AudioFormat};

let voice = DeepgramVoice::new("DEEPGRAM_API_KEY")?;

// 可选：选择其他模型
let voice = DeepgramVoice::new("DEEPGRAM_API_KEY")?
    .with_model("nova-2-general");

let audio = std::fs::read("recording.wav")?;
let result = voice.transcribe(&audio, &SttOptions {
    language: Some("zh".to_string()),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;
println!("转录文本: {}", result.text);
```

## Azure 语音服务

Azure 认知服务语音同时支持 TTS 和 STT。启用 `azure` feature，需要设置 `AZURE_SPEECH_KEY` 和 `AZURE_SPEECH_REGION` 环境变量。

```rust,ignore
use synaptic::voice::azure::AzureSpeechVoice;
use synaptic::voice::{TtsProvider, SttProvider, TtsOptions, SttOptions, AudioFormat};

let voice = AzureSpeechVoice::new("AZURE_SPEECH_KEY", "AZURE_SPEECH_REGION")?;

// 文本转语音
let audio = voice.synthesize("你好，Azure！", &TtsOptions {
    voice: "zh-CN-XiaoxiaoNeural".to_string(),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;

// 语音转文本
let result = voice.transcribe(&audio, &SttOptions {
    language: Some("zh".to_string()),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;
```

## Google Cloud 语音识别

Google Cloud STT 使用 Speech v1 REST API。启用 `google` feature，需要设置 `GOOGLE_API_KEY` 环境变量。

```rust,ignore
use synaptic::voice::google::GoogleSpeechVoice;
use synaptic::voice::{SttProvider, SttOptions, AudioFormat};

let voice = GoogleSpeechVoice::new("GOOGLE_API_KEY")?;

let audio = std::fs::read("recording.wav")?;
let result = voice.transcribe(&audio, &SttOptions {
    language: Some("zh".to_string()),
    format: AudioFormat::Wav,
    ..Default::default()
}).await?;
println!("转录文本: {}", result.text);
```

## 流式 TTS

对于低延迟音频播放，使用 `StreamingTtsProvider`，它会在音频块可用时立即返回，而不是等待整个响应缓冲完成。

```rust,ignore
use futures::StreamExt;
use synaptic::voice::{StreamingTtsProvider, TtsOptions};
use synaptic::voice::openai::OpenAiVoice;

let voice = OpenAiVoice::new("OPENAI_API_KEY")?;
let options = TtsOptions::default();

let mut stream = voice.synthesize_stream("你好，流式语音！", &options).await?;

while let Some(chunk) = stream.next().await {
    let bytes = chunk?;
    // 将音频块写入输出设备或文件
    println!("接收到 {} 字节", bytes.len());
}
```

`OpenAiVoice` 和 `ElevenLabsVoice` 均实现了 `StreamingTtsProvider`。该 trait 继承自 `TtsProvider`，因此流式 provider 也支持一次性的 `synthesize()` 方法。

### 实现自定义流式 Provider

```rust,ignore
use async_trait::async_trait;
use synaptic::core::SynapticError;
use synaptic::voice::{StreamingTtsProvider, TtsProvider, TtsOptions, TtsStream};

struct MyStreamingTts { /* ... */ }

#[async_trait]
impl TtsProvider for MyStreamingTts {
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<Vec<u8>, SynapticError> {
        // 回退方案：将流收集到缓冲区
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
        // 从你的服务返回音频块流
        todo!()
    }
}
```

## 语音活动检测 (VAD)

`VadDetector` trait 和 `EnergyVad` 实现提供语音活动检测功能 -- 在音频数据中识别语音片段。VAD 始终可用（无需 feature flag），零外部依赖。

```rust,ignore
use synaptic::voice::{EnergyVad, VadDetector, AudioFormat};

let vad = EnergyVad::default();

// 或自定义阈值
let vad = EnergyVad::default()
    .with_threshold(0.02)      // RMS 振幅阈值
    .with_frame_ms(30)         // 帧时长（毫秒）
    .with_min_speech_ms(100);  // 最小语音片段时长

let pcm_audio = std::fs::read("recording.pcm")?;
let segments = vad.detect(&pcm_audio, AudioFormat::Pcm).await?;

for seg in &segments {
    println!("语音: {:.2}s - {:.2}s (概率: {:.2})", seg.start_secs, seg.end_secs, seg.probability);
}
```

**注意:** VAD 目前仅支持 PCM16 音频格式，其他格式将返回错误。

## 配置参考

### TtsOptions

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `voice` | `String` | `"alloy"` | 语音标识符（provider 特定） |
| `format` | `AudioFormat` | `Mp3` | 输出音频格式 |
| `speed` | `f32` | `1.0` | 语速倍率（1.0 = 正常速度） |

### SttOptions

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `language` | `Option<String>` | `None` | 语言提示（ISO 639-1，如 `"zh"`、`"en"`） |
| `format` | `AudioFormat` | `Mp3` | 输入音频格式 |
| `prompt` | `Option<String>` | `None` | 可选的引导提示，帮助提高转录准确性 |

### AudioFormat 枚举

```rust,ignore
pub enum AudioFormat {
    Mp3,   // audio/mpeg
    Wav,   // audio/wav
    Ogg,   // audio/ogg
    Flac,  // audio/flac
    Pcm,   // audio/pcm
}
```

每个变体提供 `mime_type()` 和 `extension()` 方法：

```rust,ignore
let format = AudioFormat::Wav;
assert_eq!(format.mime_type(), "audio/wav");
assert_eq!(format.extension(), "wav");
```

## 自定义 Provider

实现 `TtsProvider` 或 `SttProvider` trait 以接入自定义语音服务：

```rust,ignore
use async_trait::async_trait;
use synaptic::core::SynapticError;
use synaptic::voice::{TtsProvider, TtsOptions, AudioFormat};

pub struct MyTtsProvider {
    endpoint: String,
}

#[async_trait]
impl TtsProvider for MyTtsProvider {
    async fn synthesize(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> Result<Vec<u8>, SynapticError> {
        // 调用自定义语音合成 API
        let client = reqwest::Client::new();
        let resp = client
            .post(&self.endpoint)
            .json(&serde_json::json!({
                "text": text,
                "voice": options.voice,
                "format": options.format.extension(),
            }))
            .send()
            .await
            .map_err(|e| SynapticError::Model(format!("TTS error: {}", e)))?;

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| SynapticError::Model(format!("TTS read error: {}", e)))
    }

    async fn list_voices(&self) -> Result<Vec<String>, SynapticError> {
        Ok(vec!["default".to_string(), "narrator".to_string()])
    }
}
```
