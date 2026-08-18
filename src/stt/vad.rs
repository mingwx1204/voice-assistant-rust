/// stt/vad.rs — Silero VAD 语音活动检测
/// =======================================
/// 使用能量检测作为基础 VAD（不依赖 ONNX Runtime）。
use anyhow::Result;

/// VAD 检测结果
#[derive(Debug, Clone)]
pub struct VadResult {
    pub is_speech: bool,
    pub probability: f32,
}

/// 能量 VAD（简单但有效的语音活动检测）
pub struct SileroVad {
    #[allow(dead_code)]
    sample_rate: u32,
    threshold: f32,
    energy_threshold: f32,
}

impl SileroVad {
    /// 创建 VAD 实例
    pub fn new(_model_path: &std::path::Path, threshold: f32, sample_rate: u32) -> Result<Self> {
        tracing::info!(
            "VAD ready (method: energy, threshold: {}, sample_rate: {})",
            threshold,
            sample_rate
        );

        Ok(Self {
            sample_rate,
            threshold,
            energy_threshold: 300.0,
        })
    }

    /// 检测一段音频中是否有语音
    pub fn detect(&mut self, audio: &[f32]) -> Result<VadResult> {
        if audio.is_empty() {
            return Ok(VadResult {
                is_speech: false,
                probability: 0.0,
            });
        }

        // 计算 RMS 能量
        let sum_squares: f64 = audio.iter().map(|&s| s as f64 * s as f64).sum();
        let rms = (sum_squares / audio.len() as f64).sqrt() as f32;

        // 归一化概率
        let probability = (rms / self.energy_threshold).min(1.0);

        Ok(VadResult {
            is_speech: rms > self.energy_threshold,
            probability,
        })
    }

    /// 重置内部状态
    pub fn reset(&mut self) {}

    /// 设置阈值
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }
}
