/// config.rs — 全局配置参数
/// ============================
/// 所有可调参数集中管理

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 音频参数
    pub audio: AudioConfig,
    /// 语音识别 (STT) 参数
    pub stt: SttConfig,
    /// LLM 推理参数
    pub llm: LlmConfig,
    /// 语音合成 (TTS) 参数
    pub tts: TtsConfig,
    /// 记忆系统参数
    pub memory: MemoryConfig,
    /// Agent 参数
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub block_size: usize,
    pub record_duration_secs: f32,
    pub continuous_timeout_secs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    pub model_path: PathBuf,
    pub language: String,
    pub beam_size: i32,
    pub use_gpu: bool,
    /// VAD: 最小语音持续时间 (秒)
    pub vad_min_speech_duration: f32,
    /// VAD: 最大语音持续时间 (秒)
    pub vad_max_speech_duration: f32,
    /// VAD: 语音概率阈值 (0.0-1.0)
    pub vad_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// llama.cpp server base URL
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Piper 模型目录
    pub model_dir: PathBuf,
    /// 说话人 ID (多说话人模型)
    pub speaker_id: Option<u32>,
    /// 语速缩放
    pub length_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub db_path: PathBuf,
    pub short_memory_turns: usize,
    pub extract_interval: usize,
    pub top_k: usize,
    pub rrf_k: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub personality: String,
    pub max_reply_sentences: usize,
    /// 提醒检查间隔 (秒)
    pub reminder_check_interval: u64,
    pub max_reminders: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("voice-assistant");

        Self {
            audio: AudioConfig {
                sample_rate: 16000,
                channels: 1,
                block_size: 512,
                record_duration_secs: 5.0,
                continuous_timeout_secs: 10.0,
            },
            stt: SttConfig {
                model_path: PathBuf::from("models/ggml-base.bin"),
                language: "zh".to_string(),
                beam_size: 5,
                use_gpu: true,
                vad_min_speech_duration: 0.3,
                vad_max_speech_duration: 3.0,
                vad_threshold: 0.5,
            },
            llm: LlmConfig {
                base_url: "http://127.0.0.1:8080/v1".to_string(),
                model: "minicpm-v".to_string(),
                api_key: "not-needed".to_string(),
                timeout_secs: 30,
                max_tokens: 512,
                temperature: 0.7,
            },
            tts: TtsConfig {
                model_dir: data_dir.join("tts-models"),
                speaker_id: None,
                length_scale: 1.0,
            },
            memory: MemoryConfig {
                db_path: data_dir.join("memory.db"),
                short_memory_turns: 10,
                extract_interval: 3,    // 每 3 轮提炼一次记忆
                top_k: 5,
                rrf_k: 60.0,
            },
            agent: AgentConfig {
                name: "Mini".to_string(),
                personality: "友好、简洁、偶尔幽默的语音助手".to_string(),
                max_reply_sentences: 3,
                reminder_check_interval: 1,
                max_reminders: 20,
            },
        }
    }
}

impl AppConfig {
    /// 从文件加载配置，不存在则使用默认值
    pub fn load() -> Self {
        let config_path = std::env::current_dir()
            .unwrap_or_default()
            .join("config.json");

        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => {
                        tracing::info!("Loaded config from {:?}", config_path);
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse config: {}, using defaults", e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config: {}, using defaults", e);
                }
            }
        }

        let config = Self::default();
        config.save();
        config
    }

    /// 保存配置到文件
    pub fn save(&self) {
        let config_path = std::env::current_dir()
            .unwrap_or_default()
            .join("config.json");

        if let Ok(content) = serde_json::to_string_pretty(&self) {
            let _ = std::fs::write(&config_path, content);
            tracing::info!("Config saved to {:?}", config_path);
        }
    }
}
