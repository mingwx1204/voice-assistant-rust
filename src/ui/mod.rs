/// ui/mod.rs — 用户界面
/// =======================
/// egui 即时模式 GUI。

use eframe::egui;

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// 应用状态
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// 空闲，等待唤醒
    Idle,
    /// 正在监听
    Listening,
    /// 正在识别
    Transcribing,
    /// 正在思考
    Thinking,
    /// 正在播放
    Speaking,
    /// 错误
    Error(String),
}

/// 主应用
pub struct VoiceAssistantApp {
    /// 应用状态
    pub state: AppState,
    /// 聊天历史
    pub chat_history: Vec<ChatMessage>,
    /// 用户输入
    pub user_input: String,
    /// 状态消息
    pub status_message: String,
    /// 是否显示设置
    pub show_settings: bool,
    /// 唤醒词
    pub wake_word: String,
    /// 连续对话模式
    pub continuous_mode: bool,
    /// 音量指示 (0.0 - 1.0)
    pub volume_level: f32,
}

impl Default for VoiceAssistantApp {
    fn default() -> Self {
        Self {
            state: AppState::Idle,
            chat_history: Vec::new(),
            user_input: String::new(),
            status_message: "就绪".to_string(),
            show_settings: false,
            wake_word: "Hey Mini".to_string(),
            continuous_mode: true,
            volume_level: 0.0,
        }
    }
}

impl VoiceAssistantApp {
    /// 创建新应用
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加聊天消息
    pub fn add_message(&mut self, role: &str, content: &str) {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();
        self.chat_history.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp,
        });
    }

    /// 更新状态
    pub fn set_state(&mut self, state: AppState) {
        self.state = state.clone();
        self.status_message = match &state {
            AppState::Idle => "就绪".to_string(),
            AppState::Listening => "正在监听...".to_string(),
            AppState::Transcribing => "正在识别语音...".to_string(),
            AppState::Thinking => "正在思考...".to_string(),
            AppState::Speaking => "正在播放...".to_string(),
            AppState::Error(msg) => format!("错误: {}", msg),
        };
    }
}

impl eframe::App for VoiceAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("文件", |ui| {
                    if ui.button("退出").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("设置", |ui| {
                    if ui.button("打开设置").clicked() {
                        self.show_settings = !self.show_settings;
                        ui.close_menu();
                    }
                });
                ui.menu_button("帮助", |ui| {
                    ui.label("Mini 语音助手 v0.1.0");
                    ui.label("基于 Rust + egui + whisper-rs + piper");
                });
            });
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 状态指示灯
                let (color, label) = match &self.state {
                    AppState::Idle => (egui::Color32::GRAY, "● 就绪"),
                    AppState::Listening => (egui::Color32::GREEN, "● 监听中"),
                    AppState::Transcribing => (egui::Color32::YELLOW, "● 识别中"),
                    AppState::Thinking => (egui::Color32::BLUE, "● 思考中"),
                    AppState::Speaking => (egui::Color32::PURPLE, "● 播放中"),
                    AppState::Error(_) => (egui::Color32::RED, "● 错误"),
                };

                ui.colored_label(color, label);
                ui.separator();
                ui.label(&self.status_message);

                // 音量条
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("音量: {:.0}%", self.volume_level * 100.0));
                    let _response = ui.add(
                        egui::ProgressBar::new(self.volume_level)
                            .text("")
                            .animate(self.state == AppState::Listening),
                    );
                });
            });
        });

        // 左侧面板
        egui::SidePanel::left("side_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Mini 语音助手");
                ui.separator();

                ui.label("状态:");
                ui.label(&self.status_message);
                ui.separator();

                ui.label("唤醒词:");
                ui.text_edit_singleline(&mut self.wake_word);
                ui.separator();

                ui.checkbox(&mut self.continuous_mode, "连续对话模式");
                ui.separator();

                ui.label("命令:");
                ui.label("'Hey Mini' - 唤醒");
                ui.label("'退出' - 退出程序");
                ui.separator();

                // 操作按钮
                ui.horizontal(|ui| {
                    let listen_btn = ui.add_enabled(
                        self.state == AppState::Idle,
                        egui::Button::new("开始监听"),
                    );
                    if listen_btn.clicked() {
                        // TODO: 触发开始监听
                    }

                    let stop_btn = ui.add_enabled(
                        self.state != AppState::Idle && self.state != AppState::Error(String::new()),
                        egui::Button::new("停止"),
                    );
                    if stop_btn.clicked() {
                        // TODO: 触发停止
                    }
                });
            });

        // 主聊天区域
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("对话");
            ui.separator();

            // 聊天历史滚动区域
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &self.chat_history {
                        let is_user = msg.role == "user";
                        let (color, label, align) = if is_user {
                            (
                                egui::Color32::from_rgb(100, 149, 237), // 蓝色
                                "你",
                                egui::Align::RIGHT,
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(50, 205, 50), // 绿色
                                "Mini",
                                egui::Align::LEFT,
                            )
                        };

                        ui.with_layout(egui::Layout::top_down(align), |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(color, format!("{} [{}]", label, msg.timestamp));
                            });
                            ui.label(&msg.content);
                        });
                        ui.add_space(8.0);
                    }
                });

            ui.separator();

            // 输入区域
            ui.horizontal(|ui| {
                let input_width = ui.available_width() - 80.0;
                let response = ui.add_sized(
                    [input_width, 30.0],
                    egui::TextEdit::singleline(&mut self.user_input)
                        .hint_text("输入消息... (Enter 发送)"),
                );

                let send_btn = ui.button("发送");
                let should_send = send_btn.clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                if should_send && !self.user_input.trim().is_empty() {
                    let msg = self.user_input.trim().to_string();
                    self.add_message("user", &msg);
                    self.user_input.clear();
                    // TODO: 触发处理
                }
            });
        });

        // 设置窗口
        if self.show_settings {
            egui::Window::new("设置")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("音频设置");
                    ui.label("采样率: 16000 Hz");
                    ui.label("声道: 单声道");
                    ui.separator();

                    ui.heading("语音识别");
                    ui.label("模型: whisper base");
                    ui.label("语言: 中文");
                    ui.separator();

                    ui.heading("语音合成");
                    ui.label("引擎: Piper TTS");
                    ui.separator();

                    ui.heading("LLM");
                    ui.label("服务: llama.cpp");
                    ui.label("地址: http://127.0.0.1:8080/v1");
                    ui.separator();

                    if ui.button("关闭").clicked() {
                        self.show_settings = false;
                    }
                });
        }
    }
}
