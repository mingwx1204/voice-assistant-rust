/// stt/whisper.rs — Whisper 语音识别
/// ====================================
/// 基于 whisper-rs (whisper.cpp) 的本地语音识别。
use anyhow::{Context, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// 语音识别引擎
pub struct WhisperStt {
    context: WhisperContext,
    language: String,
    beam_size: i32,
    #[allow(dead_code)]
    use_gpu: bool,
}

impl WhisperStt {
    /// 创建 STT 实例
    pub fn new(model_path: &Path, language: &str, beam_size: i32, use_gpu: bool) -> Result<Self> {
        let model_path_str = model_path.to_str().context("Invalid model path")?;

        tracing::info!(
            "Loading Whisper model: {} (lang: {}, beam: {}, gpu: {})",
            model_path_str,
            language,
            beam_size,
            use_gpu
        );

        let params = WhisperContextParameters {
            use_gpu,
            ..Default::default()
        };

        let context = WhisperContext::new_with_params(model_path_str, params)
            .context("Failed to load Whisper model")?;

        tracing::info!("Whisper model loaded successfully");

        Ok(Self {
            context,
            language: language.to_string(),
            beam_size,
            use_gpu,
        })
    }

    /// 识别音频数据
    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        let mut state = self.context.create_state()?;

        let mut params = if self.beam_size > 1 {
            FullParams::new(SamplingStrategy::BeamSearch {
                beam_size: self.beam_size,
                patience: 1.0,
            })
        } else {
            FullParams::new(SamplingStrategy::Greedy { best_of: 1 })
        };

        // 设置语言
        params.set_language(Some(&self.language));

        // 关闭一些不需要的功能以提高速度
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // 运行推理
        state.full(params, audio)?;

        // 获取结果
        let num_segments = state.full_n_segments()?;
        let mut text = String::new();

        for i in 0..num_segments {
            if let Ok(segment_text) = state.full_get_segment_text(i) {
                let trimmed = segment_text.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
        }

        let text = text.trim().to_string();
        if !text.is_empty() {
            tracing::info!("STT result: {}", text);
        }

        Ok(text)
    }

    /// 快速识别（用于唤醒词检测，使用更小的 beam size）
    pub fn transcribe_quick(&self, audio: &[f32]) -> Result<String> {
        let mut state = self.context.create_state()?;

        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        state.full(params, audio)?;

        let num_segments = state.full_n_segments()?;
        let mut text = String::new();

        for i in 0..num_segments {
            if let Ok(segment_text) = state.full_get_segment_text(i) {
                let trimmed = segment_text.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
        }

        Ok(text.trim().to_string())
    }
}
