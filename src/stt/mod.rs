/// stt/mod.rs — 语音识别子系统
/// ================================
/// 统一管理 VAD 和 Whisper STT。

pub mod vad;
pub mod whisper;

use anyhow::Result;
use std::path::Path;

use crate::config::SttConfig;
use vad::SileroVad;
use whisper::WhisperStt;

/// 完整的语音识别管道：VAD → Whisper
pub struct SpeechToText {
    vad: SileroVad,
    whisper: WhisperStt,
    min_speech_duration: f32,
    max_speech_duration: f32,
}

impl SpeechToText {
    /// 创建 STT 管道
    pub fn new(config: &SttConfig, silero_model_path: &Path) -> Result<Self> {
        let vad = SileroVad::new(silero_model_path, config.vad_threshold, 16000)?;
        let whisper = WhisperStt::new(
            &config.model_path,
            &config.language,
            config.beam_size,
            config.use_gpu,
        )?;

        Ok(Self {
            vad,
            whisper,
            min_speech_duration: config.vad_min_speech_duration,
            max_speech_duration: config.vad_max_speech_duration,
        })
    }

    /// 完整识别流程：VAD 检测 → Whisper 转写
    ///
    /// # Arguments
    /// * `audio` - f32 归一化音频数据 (16kHz)
    ///
    /// # Returns
    /// 识别出的文本，如果没有有效语音则返回 None
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<Option<String>> {
        // 计算音频时长
        let duration = audio.len() as f32 / 16000.0;

        // VAD 检测
        let vad_result = self.vad.detect(audio)?;

        if !vad_result.is_speech {
            tracing::debug!(
                "VAD: no speech detected (prob: {:.3})",
                vad_result.probability
            );
            return Ok(None);
        }

        // 检查语音时长
        if duration < self.min_speech_duration {
            tracing::debug!(
                "Speech too short: {:.2}s < {:.2}s",
                duration,
                self.min_speech_duration
            );
            return Ok(None);
        }

        if duration > self.max_speech_duration {
            tracing::debug!(
                "Speech too long: {:.2}s > {:.2}s, truncating",
                duration,
                self.max_speech_duration
            );
            // 截断到最大时长
            let max_samples = (self.max_speech_duration * 16000.0) as usize;
            let audio = &audio[..max_samples.min(audio.len())];
            let text = self.whisper.transcribe(audio)?;
            return Ok(Some(text));
        }

        // Whisper 转写
        let text = self.whisper.transcribe(audio)?;

        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    /// 快速识别（用于唤醒词检测）
    pub fn transcribe_quick(&self, audio: &[f32]) -> Result<String> {
        self.whisper.transcribe_quick(audio)
    }

    /// 重置 VAD 状态
    pub fn reset_vad(&mut self) {
        self.vad.reset();
    }
}
