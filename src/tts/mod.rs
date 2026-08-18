/// tts/mod.rs — 语音合成子系统
/// =================================
/// 支持流式 TTS 播放。
pub mod piper;

use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::config::TtsConfig;
use piper::PiperTts;

/// 文字转语音引擎
pub struct TextToSpeech {
    engine: PiperTts,
    is_playing: Arc<Mutex<bool>>,
}

impl TextToSpeech {
    /// 创建 TTS 引擎
    pub fn new(config: &TtsConfig) -> Result<Self> {
        let engine = PiperTts::new(&config.model_dir, config.speaker_id, config.length_scale)?;

        tracing::info!("TTS ready (sample_rate: {})", engine.sample_rate(),);

        Ok(Self {
            engine,
            is_playing: Arc::new(Mutex::new(false)),
        })
    }

    /// 合成语音并返回 WAV 字节数据
    pub fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let clean_text = self.clean_text(text);
        if clean_text.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!("Synthesizing: {}", &clean_text[..clean_text.len().min(30)]);
        let wav_data = self.engine.synthesize(&clean_text)?;

        Ok(wav_data)
    }

    /// 流式合成 — 分句合成并回调
    ///
    /// 将文本按句子分割，每合成一句就回调一次。
    pub fn synthesize_streaming(
        &self,
        text: &str,
        mut on_chunk: impl FnMut(Vec<u8>),
    ) -> Result<()> {
        let clean_text = self.clean_text(text);
        if clean_text.is_empty() {
            return Ok(());
        }

        // 按句子分割
        let sentences = self.split_sentences(&clean_text);

        for sentence in &sentences {
            if sentence.trim().is_empty() {
                continue;
            }

            tracing::debug!("TTS chunk: {}", &sentence[..sentence.len().min(30)]);
            match self.engine.synthesize(sentence) {
                Ok(wav_data) => {
                    on_chunk(wav_data);
                }
                Err(e) => {
                    tracing::warn!("TTS chunk failed: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 按句子分割文本
    fn split_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();

        for ch in text.chars() {
            current.push(ch);
            // 中文句号、问号、感叹号、英文句号等
            if (ch == '。'
                || ch == '！'
                || ch == '？'
                || ch == '.'
                || ch == '!'
                || ch == '?'
                || ch == '，'
                || ch == ',')
                && current.trim().len() > 2
            {
                sentences.push(std::mem::take(&mut current));
            }
        }

        if !current.trim().is_empty() {
            sentences.push(current);
        }

        sentences
    }

    /// 清理文本（去除 markdown 格式）
    fn clean_text(&self, text: &str) -> String {
        let mut text = text.to_string();
        text = text.replace("**", "");
        text = text.replace("__", "");
        let cleaned: String = text.chars().filter(|c| *c != '*' && *c != '_').collect();
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        parts.join(" ").trim().to_string()
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> u32 {
        self.engine.sample_rate()
    }

    /// 标记播放状态
    pub fn set_playing(&self, playing: bool) {
        *self.is_playing.lock().unwrap() = playing;
    }

    pub fn is_playing(&self) -> bool {
        *self.is_playing.lock().unwrap()
    }
}
