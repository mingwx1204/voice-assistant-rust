/// agent/orchestrator.rs — Agent 编排器
/// =======================================
/// 核心编排：协调 录音 → VAD → STT → Agent → TTS → 播放 完整对话流程。
///
/// 三大特性：
/// 1. 多轮连续对话 — 回复后继续监听，不用反复唤醒
/// 2. 流式响应 — LLM 边生成边播放
/// 3. 记忆提炼 — 定期用 LLM 提取关键信息存入长期记忆
use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::agent::{AgentPersona, ToolRegistry};
use crate::audio::{AudioCapture, AudioPlayback};
use crate::config::AppConfig;
use crate::llm::{ChatMessage, LlmClient};
use crate::memory::{KnowledgeBase, MemoryDatabase};
use crate::stt::SpeechToText;
use crate::tts::TextToSpeech;
use crate::ui::{AppState, VoiceAssistantApp};

/// Agent 编排器
pub struct AgentOrchestrator {
    config: AppConfig,
    session_id: String,

    // 组件
    stt: Option<SpeechToText>,
    tts: Option<TextToSpeech>,
    llm: Option<LlmClient>,
    db: Option<Arc<Mutex<MemoryDatabase>>>,
    kb: Option<Arc<Mutex<KnowledgeBase>>>,
    tools: ToolRegistry,
    persona: AgentPersona,

    // 音频
    capture: Option<AudioCapture>,
    playback: Option<AudioPlayback>,

    // 状态
    running: bool,
    turn_count: usize,

    // 对话历史
    chat_history: Vec<ChatMessage>,
    max_history_turns: usize,

    // 剪贴板监听
    clipboard_monitoring: bool,

    // UI
    app_state: Arc<Mutex<VoiceAssistantApp>>,
}

impl AgentOrchestrator {
    /// 创建编排器
    pub fn new(config: AppConfig, app_state: Arc<Mutex<VoiceAssistantApp>>) -> Self {
        let session_id = format!("session_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let persona = AgentPersona::from_config(&config.agent);
        let tools = ToolRegistry::new();
        let max_history_turns = config.memory.short_memory_turns;

        Self {
            config,
            session_id,
            stt: None,
            tts: None,
            llm: None,
            db: None,
            kb: None,
            tools,
            persona,
            capture: None,
            playback: None,
            running: false,
            turn_count: 0,
            chat_history: Vec::new(),
            max_history_turns,
            clipboard_monitoring: true,
            app_state,
        }
    }

    /// 初始化所有组件
    pub fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing components...");

        // 初始化记忆系统
        let db = MemoryDatabase::new(&self.config.memory.db_path)?;
        let db_arc = Arc::new(Mutex::new(db));
        self.db = Some(Arc::clone(&db_arc));
        self.tools.set_database(Arc::clone(&db_arc));

        // 初始化知识库
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("voice-assistant");
        match KnowledgeBase::new(&data_dir) {
            Ok(kb) => {
                tracing::info!(
                    "Knowledge base: {} documents, {} chunks",
                    kb.document_count(),
                    kb.chunk_count()
                );
                self.kb = Some(Arc::new(Mutex::new(kb)));
            }
            Err(e) => {
                tracing::warn!("Knowledge base init failed: {}", e);
            }
        }

        // 初始化音频
        match AudioCapture::new(
            self.config.audio.sample_rate,
            self.config.audio.channels,
            self.config.audio.block_size,
        ) {
            Ok(capture) => {
                self.capture = Some(capture);
                tracing::info!("Audio capture ready");
            }
            Err(e) => {
                tracing::warn!("Audio capture init failed: {}", e);
            }
        }

        match AudioPlayback::new(self.config.audio.sample_rate, self.config.audio.channels) {
            Ok(playback) => {
                self.playback = Some(playback);
                tracing::info!("Audio playback ready");
            }
            Err(e) => {
                tracing::warn!("Audio playback init failed: {}", e);
            }
        }

        // 初始化 STT
        let silero_model = Path::new("models/silero_vad.onnx");
        if silero_model.exists() {
            match SpeechToText::new(&self.config.stt, silero_model) {
                Ok(stt) => {
                    self.stt = Some(stt);
                    tracing::info!("STT ready");
                }
                Err(e) => {
                    tracing::warn!("STT init failed: {}", e);
                }
            }
        } else {
            tracing::warn!("Silero VAD model not found at {:?}", silero_model);
        }

        // 初始化 TTS
        if self.config.tts.model_dir.exists() {
            match TextToSpeech::new(&self.config.tts) {
                Ok(tts) => {
                    self.tts = Some(tts);
                    tracing::info!("TTS ready");
                }
                Err(e) => {
                    tracing::warn!("TTS init failed: {}", e);
                }
            }
        } else {
            tracing::warn!("TTS model dir not found at {:?}", self.config.tts.model_dir);
        }

        // 初始化 LLM
        match LlmClient::new(&self.config.llm) {
            Ok(llm) => {
                self.llm = Some(llm);
                tracing::info!("LLM ready");
            }
            Err(e) => {
                tracing::warn!("LLM init failed: {}", e);
            }
        }

        tracing::info!("All components initialized");
        Ok(())
    }

    /// 运行主循环
    pub fn run(&mut self) -> Result<()> {
        self.running = true;
        self.print_banner();

        // 加载历史对话到上下文
        self.load_history();

        while self.running {
            // ===== 唤醒词检测 =====
            self.wait_for_wake()?;
            if !self.running {
                break;
            }

            // ===== 录音 → 识别 =====
            let audio = self.do_record()?;
            if audio.is_empty() {
                continue;
            }

            let user_text = match self.do_transcribe(&audio)? {
                Some(text) => text,
                None => {
                    self.speak(&self.persona.get_no_voice_response());
                    continue;
                }
            };

            // 保存用户消息到数据库
            if let Some(ref db) = self.db {
                let _ = db
                    .lock()
                    .unwrap()
                    .save_conversation(&self.session_id, "user", &user_text);
            }

            // 更新 UI
            {
                let mut app = self.app_state.lock().unwrap();
                app.add_message("user", &user_text);
            }

            tracing::info!("User: {}", user_text);

            // ===== 处理请求 =====
            let response = self.process_request(&user_text);

            // 保存助手回复到数据库
            if let Some(ref db) = self.db {
                let _ =
                    db.lock()
                        .unwrap()
                        .save_conversation(&self.session_id, "assistant", &response);
            }

            // 更新 UI
            {
                let mut app = self.app_state.lock().unwrap();
                app.add_message("assistant", &response);
            }

            tracing::info!("Mini: {}", response);

            // ===== 更新对话历史 =====
            self.chat_history.push(ChatMessage {
                role: "user".to_string(),
                content: user_text,
            });
            self.chat_history.push(ChatMessage {
                role: "assistant".to_string(),
                content: response.clone(),
            });
            self.trim_history();

            // ===== 播放回复 =====
            self.speak(&response);

            self.turn_count += 1;

            // ===== 记忆提炼 =====
            if self.turn_count > 0
                && self
                    .turn_count
                    .is_multiple_of(self.config.memory.extract_interval)
            {
                self.extract_memories();
            }

            // ===== 剪贴板监听 =====
            if self.clipboard_monitoring {
                if let Some(clip_text) = self.tools.check_clipboard_change() {
                    tracing::info!(
                        "Clipboard changed: {}",
                        &clip_text[..clip_text.len().min(50)]
                    );
                }
            }

            // ===== 连续对话：回到监听，不退出 =====
            tracing::info!("Continuing conversation... (turn {})", self.turn_count);
        }

        self.cleanup();
        Ok(())
    }

    /// 等待唤醒词
    fn wait_for_wake(&mut self) -> Result<()> {
        {
            let mut app = self.app_state.lock().unwrap();
            app.set_state(AppState::Listening);
        }

        // 第一轮需要唤醒词，后续连续对话直接跳过
        if self.turn_count > 0 {
            let timeout = self.config.audio.continuous_timeout_secs;
            tracing::info!("Continuously listening ({}s timeout)...", timeout);

            // TODO: 实现真正的 Silero VAD 唤醒词检测
            // 目前简化：等待超时时间，模拟"监听中"
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_secs_f32(timeout) && self.running {
                std::thread::sleep(Duration::from_millis(100));
            }

            if start.elapsed() >= Duration::from_secs_f32(timeout) {
                tracing::info!("Continuous conversation timeout, resetting");
                self.turn_count = 0; // 重置，下次需要重新唤醒
            }
        } else {
            tracing::info!("Listening for wake word...");
            // TODO: 真正的唤醒词检测
            std::thread::sleep(Duration::from_secs(1));
        }

        Ok(())
    }

    /// 录音
    fn do_record(&mut self) -> Result<Vec<f32>> {
        {
            let mut app = self.app_state.lock().unwrap();
            app.set_state(AppState::Listening);
        }

        let duration = self.config.audio.record_duration_secs;
        tracing::info!("Recording ({:.1}s)...", duration);

        match &mut self.capture {
            Some(capture) => {
                // 启动音量更新线程
                let volume = capture.volume.clone();
                let app_state = self.app_state.clone();
                let vol_thread = std::thread::spawn(move || loop {
                    let vol = *volume.lock().unwrap();
                    let mut app = app_state.lock().unwrap();
                    app.volume_level = vol;
                    std::thread::sleep(std::time::Duration::from_millis(50));
                });

                let result = capture.record_blocking(duration);

                // 停止音量更新线程
                drop(vol_thread);

                result
            }
            None => {
                tracing::warn!("No audio capture available");
                Ok(Vec::new())
            }
        }
    }

    /// 语音识别
    fn do_transcribe(&mut self, audio: &[f32]) -> Result<Option<String>> {
        {
            let mut app = self.app_state.lock().unwrap();
            app.set_state(AppState::Transcribing);
        }

        match &mut self.stt {
            Some(stt) => Ok(stt.transcribe(audio)?),
            None => {
                tracing::warn!("STT not available");
                Ok(None)
            }
        }
    }

    /// 处理用户请求
    fn process_request(&mut self, user_text: &str) -> String {
        {
            let mut app = self.app_state.lock().unwrap();
            app.set_state(AppState::Thinking);
        }

        // 检查工具调用
        if let Some(tool_result) = self.tools.detect_and_execute(user_text) {
            if tool_result.should_respond {
                // 如果需要 LLM 处理（如翻译）
                if tool_result.needs_llm {
                    if let Some(prompt) = &tool_result.llm_prompt {
                        return self.call_llm(prompt, None);
                    }
                }
                // 搜索结果用 LLM 摘要
                if tool_result.output.starts_with("搜索结果：") {
                    return self.summarize_search(user_text, &tool_result.output);
                }
                return tool_result.output;
            }
        }

        // 预先准备数据，避免借用冲突
        let memory_context = self.build_memory_context(user_text);
        let system_prompt = self.persona.get_system_prompt(&memory_context);
        let history = self.chat_history.clone();
        let app_state_clone = self.app_state.clone();

        // 调用 LLM（带流式输出）
        if let Some(ref mut llm) = self.llm {
            if llm.is_available() {
                let response =
                    llm.chat_stream(user_text, Some(&system_prompt), &history, |token| {
                        let mut app = app_state_clone.lock().unwrap();
                        app.status_message = format!("Mini: {}", token);
                    });

                match response {
                    Ok(text) => return text,
                    Err(e) => {
                        tracing::error!("LLM call failed: {}", e);
                        return self.persona.get_error_response();
                    }
                }
            } else {
                return self.persona.get_service_unavailable_response();
            }
        }

        tracing::warn!("LLM not available");
        self.persona.get_service_unavailable_response()
    }

    /// 构建记忆上下文（包含 RAG 知识库）
    fn build_memory_context(&self, query: &str) -> String {
        let mut context = String::new();

        // 1. 从知识库检索相关文档
        if let Some(ref kb) = self.kb {
            let kb = kb.lock().unwrap();
            let rag_results = kb.search(query, 3);
            if !rag_results.is_empty() {
                context.push_str("相关知识：\n");
                for (content, score) in &rag_results {
                    context.push_str(&format!("- [相关度:{:.1}] {}\n", score, content));
                }
                context.push('\n');
            }
        }

        // 2. 从记忆数据库检索
        if let Some(ref db) = self.db {
            let db = db.lock().unwrap();
            if let Ok(memories) = db.search_memories_fts(query, 3) {
                if !memories.is_empty() {
                    context.push_str("相关记忆：\n");
                    for (_, content, category, _) in &memories {
                        context.push_str(&format!("- [{}] {}\n", category, content));
                    }
                    context.push('\n');
                }
            }

            // 最近对话
            if let Ok(conversations) = db.get_recent_conversations(&self.session_id, 6) {
                if !conversations.is_empty() {
                    context.push_str("最近对话：\n");
                    for (role, content, _) in &conversations {
                        let prefix = if role == "user" { "用户" } else { "助手" };
                        context.push_str(&format!("{}: {}\n", prefix, content));
                    }
                }
            }
        }

        context
    }

    /// 播放语音
    fn speak(&mut self, text: &str) {
        {
            let mut app = self.app_state.lock().unwrap();
            app.set_state(AppState::Speaking);
        }

        if let Some(ref tts) = self.tts {
            tts.set_playing(true);
            let sample_rate = tts.sample_rate();
            let config_sample_rate = self.config.audio.sample_rate;

            // 流式合成并播放
            let _ = tts.synthesize_streaming(text, |wav_data| {
                if let Ok(samples) = Self::wav_to_samples(&wav_data) {
                    let resampled = if sample_rate != config_sample_rate {
                        Self::resample_static(&samples, sample_rate, config_sample_rate)
                    } else {
                        samples
                    };
                    // 播放每个 chunk
                    if let Some(ref playback) = self.playback {
                        if let Ok(handle) = playback.play(&resampled) {
                            handle.wait_or_interrupt(Duration::from_millis(50));
                        }
                    }
                }
            });

            tts.set_playing(false);
        } else {
            tracing::warn!("TTS not available");
            std::thread::sleep(Duration::from_millis(500));
        }

        {
            let mut app = self.app_state.lock().unwrap();
            app.set_state(AppState::Idle);
        }
    }

    /// WAV 字节 → f32 采样
    fn wav_to_samples(wav_data: &[u8]) -> Result<Vec<f32>> {
        let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_data))?;
        let spec = reader.spec();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / (1i32 << (spec.bits_per_sample - 1)) as f32))
                .collect::<Result<Vec<_>, _>>()?,
            hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        };
        Ok(samples)
    }

    /// 静态重采样
    fn resample_static(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        let ratio = from_rate as f64 / to_rate as f64;
        let output_len = (input.len() as f64 / ratio) as usize;
        let mut output = Vec::with_capacity(output_len);
        for i in 0..output_len {
            let src_idx = i as f64 * ratio;
            let idx = src_idx as usize;
            let frac = src_idx - idx as f64;
            let sample = if idx + 1 < input.len() {
                input[idx] as f64 * (1.0 - frac) + input[idx + 1] as f64 * frac
            } else if idx < input.len() {
                input[idx] as f64
            } else {
                0.0
            };
            output.push(sample as f32);
        }
        output
    }

    /// ===== 通用 LLM 调用 =====
    fn call_llm(&mut self, prompt: &str, system: Option<&str>) -> String {
        let Some(ref mut llm) = self.llm else {
            return "LLM 不可用".to_string();
        };
        if !llm.is_available() {
            return "LLM 不可用".to_string();
        }
        let sys = system.unwrap_or("你是Mini，简洁回答。");
        let history = self.chat_history.clone();
        match llm.chat_stream(prompt, Some(sys), &history, |_| {}) {
            Ok(text) => text,
            Err(e) => format!("LLM 错误: {}", e),
        }
    }

    /// ===== 搜索摘要：用 LLM 总结搜索结果 =====
    fn summarize_search(&self, _query: &str, search_results: &str) -> String {
        // 简单返回搜索结果，让上层 LLM 处理摘要
        search_results.to_string()
    }

    /// ===== 功能3: 记忆提炼 =====
    ///
    /// 定期调用 LLM 分析最近的对话，提取关键信息存入长期记忆。
    fn extract_memories(&mut self) {
        tracing::info!("Extracting memories from conversation...");

        // 收集最近的对话
        let recent: Vec<String> = self
            .chat_history
            .iter()
            .rev()
            .take(self.config.memory.extract_interval * 2)
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect();

        if recent.is_empty() {
            return;
        }

        let conversation_text = recent.join("\n");

        // 让 LLM 提取关键信息
        let extraction_prompt = format!(
            r#"分析以下对话，提取用户提到的**关键事实和偏好**（如姓名、喜好、习惯、重要事件等）。

格式要求：
- 每行一条记忆
- 格式为 "类别: 内容"（类别如: preference, fact, personal, habit）
- 只提取确定的事实，不要推测
- 如果没有值得记住的内容，返回空

对话内容：
{}"#,
            conversation_text
        );

        if let Some(ref mut llm) = self.llm {
            if llm.is_available() {
                match llm.chat_sync(
                    &extraction_prompt,
                    Some("你是一个记忆提取助手。只输出提取的记忆列表，每行一条。"),
                    &[],
                ) {
                    Ok(extracted) => {
                        let extracted = extracted.trim();
                        if extracted.is_empty() || extracted == "无" || extracted.len() < 5 {
                            tracing::debug!("No new memories extracted");
                            return;
                        }

                        // 解析并存储
                        let mut count = 0;
                        if let Some(ref db) = self.db {
                            let db = db.lock().unwrap();
                            for line in extracted.lines() {
                                let line = line.trim();
                                if line.is_empty() {
                                    continue;
                                }
                                // 解析 "类别: 内容" 格式
                                if let Some(pos) = line.find(':') {
                                    let category = line[..pos].trim().to_string();
                                    let content = line[pos + 1..].trim().to_string();
                                    if !content.is_empty() {
                                        let _ = db.save_memory(
                                            &content,
                                            &category,
                                            Some("llm_extract"),
                                            None,
                                            0.6,
                                        );
                                        count += 1;
                                    }
                                } else {
                                    // 没有类别前缀，作为事实存储
                                    let _ = db.save_memory(
                                        line,
                                        "fact",
                                        Some("llm_extract"),
                                        None,
                                        0.5,
                                    );
                                    count += 1;
                                }
                            }
                        }

                        if count > 0 {
                            tracing::info!("Extracted {} new memories", count);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Memory extraction failed: {}", e);
                    }
                }
            }
        }
    }

    /// 裁剪对话历史，防止过长
    fn trim_history(&mut self) {
        let max_messages = self.max_history_turns * 2; // 每轮包含 user + assistant
        if self.chat_history.len() > max_messages {
            let excess = self.chat_history.len() - max_messages;
            self.chat_history.drain(..excess);
        }
    }

    /// 从数据库加载历史对话到上下文
    fn load_history(&mut self) {
        let Some(ref db) = self.db else {
            return;
        };

        let db = db.lock().unwrap();
        if let Ok(conversations) =
            db.get_recent_conversations(&self.session_id, self.max_history_turns * 2)
        {
            for (role, content, _) in conversations {
                self.chat_history.push(ChatMessage { role, content });
            }
            if !self.chat_history.is_empty() {
                tracing::info!("Loaded {} messages from history", self.chat_history.len());
            }
        }
    }

    /// 打印横幅
    fn print_banner(&self) {
        let stats = self.get_stats();
        println!(
            r#"
╔═══════════════════════════════════════════════╗
║         Mini 语音助手 v0.1.0 (Rust)          ║
║  Features: Stream | Multi-turn | Memory       ║
╠═══════════════════════════════════════════════╣
║  Agent: {} ({}...)         ║
║  Memory: {} long-term | {} conversations     ║
║  Reminders: {} pending                       ║
╠═══════════════════════════════════════════════╣
║  Say '{}' to start | Type 'exit' to quit     ║
╚═══════════════════════════════════════════════╝
"#,
            self.persona.name,
            &self.persona.personality[..self.persona.personality.len().min(15)],
            stats.memories,
            stats.conversations,
            stats.pending_reminders,
            self.config.stt.language,
        );
    }

    /// 获取统计信息
    fn get_stats(&self) -> crate::memory::MemoryStats {
        match &self.db {
            Some(db) => db.lock().unwrap().get_stats().unwrap_or_default(),
            None => crate::memory::MemoryStats::default(),
        }
    }

    /// 清理资源
    fn cleanup(&mut self) {
        tracing::info!("Shutting down...");
        let stats = self.get_stats();
        tracing::info!(
            "Turns: {} | Memories: {} | Conversations: {}",
            self.turn_count,
            stats.memories,
            stats.conversations
        );
        tracing::info!("Goodbye!");
    }

    /// 停止运行
    pub fn stop(&mut self) {
        self.running = false;
    }
}
