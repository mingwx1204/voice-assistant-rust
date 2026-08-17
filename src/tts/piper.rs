/// tts/piper.rs — Piper 语音合成
/// =================================
/// 简化的 TTS 实现（占位）。

use anyhow::Result;
use std::path::Path;

/// Piper TTS 引擎（简化版）
pub struct PiperTts {
    sample_rate: u32,
}

impl PiperTts {
    /// 创建 Piper TTS 实例
    pub fn new(_model_dir: &Path, _speaker_id: Option<u32>, _length_scale: f32) -> Result<Self> {
        tracing::info!("Piper TTS: placeholder mode (no model loaded)");
        Ok(Self {
            sample_rate: 22050,
        })
    }

    /// 合成语音（占位：返回静音）
    pub fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        tracing::debug!("TTS synthesize (placeholder): {}", &text[..text.len().min(30)]);
        // 占位：返回 0.5 秒静音
        let num_samples = self.sample_rate as usize / 2;
        Ok(vec![0.0f32; num_samples])
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 获取说话人数量
    pub fn num_speakers(&self) -> u32 {
        1
    }

    /// 设置说话人
    pub fn set_speaker(&mut self, _speaker_id: u32) {}

    /// 设置语速
    pub fn set_length_scale(&mut self, _scale: f32) {}
}
