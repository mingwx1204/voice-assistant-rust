/// tts/mod.rs — 语音合成子系统
/// =================================
/// 统一管理 Piper TTS。

pub mod piper;

use anyhow::Result;
use std::path::Path;

use crate::config::TtsConfig;
use piper::PiperTts;

/// 文字转语音引擎
pub struct TextToSpeech {
    engine: PiperTts,
    /// 是否正在播放
    is_playing: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TextToSpeech {
    /// 创建 TTS 引擎
    pub fn new(config: &TtsConfig) -> Result<Self> {
        let engine = PiperTts::new(
            &config.model_dir,
            config.speaker_id,
            config.length_scale,
        )?;

        tracing::info!(
            "TTS ready (sample_rate: {}, speakers: {})",
            engine.sample_rate(),
            engine.num_speakers()
        );

        Ok(Self {
            engine,
            is_playing: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// 合成语音并返回音频数据
    pub fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let clean_text = self.clean_text(text);
        if clean_text.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!("Synthesizing: {}", &clean_text[..clean_text.len().min(30)]);
        let audio = self.engine.synthesize(&clean_text)?;

        Ok(audio)
    }

    /// 清理文本（去除 markdown 格式）
    fn clean_text(&self, text: &str) -> String {
        let mut text = text.to_string();

        // 简单的字符串替换（避免 regex 依赖问题）
        text = text.replace("**", "");
        text = text.replace("__", "");
        // 去除单个 * 和 _
        let cleaned: String = text.chars().filter(|c| *c != '*' && *c != '_').collect();
        text = cleaned;
        // 去除多余空格
        let parts: Vec<&str> = text.split_whitespace().collect();
        text = parts.join(" ");

        text.trim().to_string()
    }

    /// 设置说话人
    pub fn set_speaker(&mut self, speaker_id: u32) {
        self.engine.set_speaker(speaker_id);
    }

    /// 设置语速
    pub fn set_length_scale(&mut self, scale: f32) {
        self.engine.set_length_scale(scale);
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> u32 {
        self.engine.sample_rate()
    }

    /// 标记开始播放
    pub fn mark_playing(&self) {
        self.is_playing
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// 标记停止播放
    pub fn mark_stopped(&self) {
        self.is_playing
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// 是否正在播放
    pub fn is_playing(&self) -> bool {
        self.is_playing
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
