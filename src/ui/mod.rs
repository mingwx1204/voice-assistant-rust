/// ui/mod.rs — 用户界面
/// =======================
/// egui 即时模式 GUI — v0.4.0

pub mod tray;
pub mod hotkey;

use eframe::egui;

/// 简单的 Markdown 渲染器
pub fn render_markdown(ui: &mut egui::Ui, text: &str) {
    for line in text.lines() {
        let line = line.trim();

        // 标题
        if line.starts_with("### ") {
            ui.heading(&line[4..]);
        } else if line.starts_with("## ") {
            ui.heading(&line[3..]);
        } else if line.starts_with("# ") {
            ui.heading(&line[2..]);
        }
        // 加粗
        else if line.starts_with("**") && line.ends_with("**") {
            let bold_text = &line[2..line.len()-2];
            ui.label(egui::RichText::new(bold_text).strong());
        }
        // 列表项
        else if line.starts_with("- ") || line.starts_with("* ") {
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label(&line[2..]);
            });
        }
        // 编号列表
        else if line.chars().next().map_or(false, |c| c.is_ascii_digit()) && line.contains(". ") {
            ui.label(line);
        }
        // 代码块
        else if line.starts_with("```") {
            // 跳过代码块标记
        }
        // 引用
        else if line.starts_with("> ") {
            let quote_text = &line[2..];
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(150, 150, 150), "│");
                ui.label(egui::RichText::new(quote_text).italics());
            });
        }
        // 分隔线
        else if line == "---" || line == "***" || line == "___" {
            ui.separator();
        }
        // 普通文本
        else if !line.is_empty() {
            // 处理行内格式
            let mut job = egui::text::LayoutJob::default();
            let words: Vec<&str> = line.split(' ').collect();
            for (i, word) in words.iter().enumerate() {
                if i > 0 {
                    job.append(" ", 0.0, egui::TextFormat::default());
                }
                if word.starts_with("**") && word.ends_with("**") {
                    job.append(&word[2..word.len()-2], 0.0, egui::TextFormat {
                        font_id: egui::FontId::proportional(14.0),
                        color: egui::Color32::WHITE,
                        ..Default::default()
                    });
                } else if word.starts_with("`") && word.ends_with("`") {
                    job.append(&word[1..word.len()-1], 0.0, egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color: egui::Color32::from_rgb(100, 200, 100),
                        background: egui::Color32::from_rgb(40, 40, 50),
                        ..Default::default()
                    });
                } else {
                    job.append(word, 0.0, egui::TextFormat::default());
                }
            }
            ui.label(job);
        }
        // 空行
        else {
            ui.add_space(4.0);
        }
    }
}

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
    Idle,
    Listening,
    Transcribing,
    Thinking,
    Speaking,
    Error(String),
}

/// 主应用
pub struct VoiceAssistantApp {
    pub state: AppState,
    pub chat_history: Vec<ChatMessage>,
    pub user_input: String,
    pub status_message: String,
    pub show_settings: bool,
    pub show_history: bool,
    pub show_help: bool,
    pub wake_word: String,
    pub continuous_mode: bool,
    pub volume_level: f32,
    pub llm_model: String,
    pub tts_voice: String,
    pub language: String,
    pub scroll_to_bottom: bool,
}

impl Default for VoiceAssistantApp {
    fn default() -> Self {
        Self {
            state: AppState::Idle,
            chat_history: Vec::new(),
            user_input: String::new(),
            status_message: "就绪".to_string(),
            show_settings: false,
            show_history: false,
            show_help: false,
            wake_word: "Hey Mini".to_string(),
            continuous_mode: true,
            volume_level: 0.0,
            llm_model: "MiniCPM-V-4.6".to_string(),
            tts_voice: "piper".to_string(),
            language: "中文".to_string(),
            scroll_to_bottom: true,
        }
    }
}

impl VoiceAssistantApp {
    pub fn new() -> Self { Self::default() }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();
        self.chat_history.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp,
        });
        self.scroll_to_bottom = true;
    }

    pub fn set_state(&mut self, state: AppState) {
        self.state = state.clone();
        self.status_message = match &state {
            AppState::Idle => "就绪".to_string(),
            AppState::Listening => "🎤 正在监听...".to_string(),
            AppState::Transcribing => "📝 正在识别语音...".to_string(),
            AppState::Thinking => "🧠 正在思考...".to_string(),
            AppState::Speaking => "🔊 正在播放...".to_string(),
            AppState::Error(msg) => format!("❌ 错误: {}", msg),
        };
    }
}

impl eframe::App for VoiceAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ===== 顶部菜单栏 =====
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("文件", |ui| {
                    if ui.button("📂 导出对话").clicked() {
                        self.add_message("system", "正在导出对话...");
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🚪 退出").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("视图", |ui| {
                    if ui.button("📜 对话历史").clicked() {
                        self.show_history = !self.show_history;
                        ui.close_menu();
                    }
                    if ui.button("⚙️ 设置").clicked() {
                        self.show_settings = !self.show_settings;
                        ui.close_menu();
                    }
                });
                ui.menu_button("帮助", |ui| {
                    if ui.button("❓ 使用帮助").clicked() {
                        self.show_help = !self.show_help;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label("Mini 语音助手 v0.1.0");
                    ui.label("Powered by Rust + egui + whisper-rs");
                });
            });
        });

        // ===== 底部状态栏 =====
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("🔊 {:.0}%", self.volume_level * 100.0));
                    ui.add(
                        egui::ProgressBar::new(self.volume_level)
                            .animate(self.state == AppState::Listening),
                    );
                });
            });
        });

        // ===== 左侧工具面板 =====
        egui::SidePanel::left("side_panel").default_width(220.0).show(ctx, |ui| {
            ui.heading("🤖 Mini 语音助手");
            ui.separator();

            // 状态卡片
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(30, 30, 40))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label("📊 系统状态");
                    ui.horizontal(|ui| {
                        ui.label("模型:");
                        ui.monospace(&self.llm_model);
                    });
                    ui.horizontal(|ui| {
                        ui.label("语言:");
                        ui.monospace(&self.language);
                    });
                    ui.horizontal(|ui| {
                        ui.label("对话:");
                        ui.monospace(format!("{} 条", self.chat_history.len()));
                    });
                });

            ui.add_space(8.0);

            // 快捷按钮
            ui.label("⚡ 快捷操作");
            ui.horizontal(|ui| {
                if ui.button("📸 截图").clicked() {
                    self.add_message("system", "正在截图...");
                }
                if ui.button("📋 剪贴板").clicked() {
                    self.user_input = "剪贴板".to_string();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("🔍 搜索").clicked() {
                    self.user_input = "搜索 ".to_string();
                }
                if ui.button("⏰ 提醒").clicked() {
                    self.user_input = "分钟后提醒我".to_string();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("💾 导出").clicked() {
                    self.user_input = "导出对话".to_string();
                }
                if ui.button("🧠 记忆").clicked() {
                    self.user_input = "记住 ".to_string();
                }
            });

            ui.add_space(8.0);

            // 系统命令
            ui.label("💻 系统命令");
            egui::ComboBox::from_id_salt("system_cmd")
                .selected_text("选择命令...")
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "🌐 打开浏览器").clicked() {
                        self.user_input = "打开浏览器".to_string();
                    }
                    if ui.selectable_label(false, "🧮 打开计算器").clicked() {
                        self.user_input = "打开计算器".to_string();
                    }
                    if ui.selectable_label(false, "📝 打开记事本").clicked() {
                        self.user_input = "打开记事本".to_string();
                    }
                    if ui.selectable_label(false, "📁 打开文件夹").clicked() {
                        self.user_input = "打开文件管理器".to_string();
                    }
                    if ui.selectable_label(false, "🔒 锁屏").clicked() {
                        self.user_input = "锁屏".to_string();
                    }
                    if ui.selectable_label(false, "⏻ 关机").clicked() {
                        self.user_input = "关机".to_string();
                    }
                });

            ui.add_space(8.0);

            // 设置
            ui.separator();
            if ui.button("⚙️ 设置").clicked() {
                self.show_settings = !self.show_settings;
            }
            if ui.button("📜 历史").clicked() {
                self.show_history = !self.show_history;
            }
            if ui.button("❓ 帮助").clicked() {
                self.show_help = !self.show_help;
            }
        });

        // ===== 主聊天区域 =====
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("💬 对话");
            ui.separator();

            // 聊天历史
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if self.chat_history.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.heading("🤖 Mini 语音助手");
                            ui.label("说 \"Hey Mini\" 或在下方输入消息开始对话");
                            ui.add_space(20.0);
                            ui.label("💡 试试说：");
                            ui.label("\"今天天气怎么样\"");
                            ui.label("\"搜索最新的新闻\"");
                            ui.label("\"记住我喜欢冰美式\"");
                            ui.label("\"5分钟后提醒我喝水\"");
                        });
                    } else {
                        for msg in &self.chat_history {
                            let is_user = msg.role == "user";
                            let (color, label) = if is_user {
                                (egui::Color32::from_rgb(100, 149, 237), "👤 你")
                            } else if msg.role == "system" {
                                (egui::Color32::from_rgb(150, 150, 150), "ℹ️ 系统")
                            } else {
                                (egui::Color32::from_rgb(50, 205, 50), "🤖 Mini")
                            };

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.colored_label(color, format!("{} [{}]", label, msg.timestamp));
                            });
                            ui.add_space(2.0);

                            let frame = egui::Frame::new()
                                .fill(if is_user {
                                    egui::Color32::from_rgb(25, 35, 50)
                                } else {
                                    egui::Color32::from_rgb(20, 30, 25)
                                })
                                .corner_radius(6.0)
                                .inner_margin(8.0);

                            frame.show(ui, |ui| {
                                // 对助手回复使用 Markdown 渲染
                                if msg.role == "assistant" {
                                    render_markdown(ui, &msg.content);
                                } else {
                                    ui.label(&msg.content);
                                }
                            });
                        }
                    }
                });

            ui.separator();

            // 输入区域
            ui.horizontal(|ui| {
                let input_width = ui.available_width() - 100.0;
                let response = ui.add_sized(
                    [input_width, 32.0],
                    egui::TextEdit::singleline(&mut self.user_input)
                        .hint_text("💬 输入消息... (Enter 发送)"),
                );

                let send_btn = ui.add_sized([80.0, 32.0], egui::Button::new("📤 发送"));
                let should_send = send_btn.clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                if should_send && !self.user_input.trim().is_empty() {
                    let msg = self.user_input.trim().to_string();
                    self.add_message("user", &msg);
                    self.user_input.clear();
                }
            });
        });

        // ===== 设置窗口 =====
        if self.show_settings {
            egui::Window::new("⚙️ 设置")
                .collapsible(false)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.heading("🤖 Agent 设置");
                    ui.horizontal(|ui| {
                        ui.label("唤醒词:");
                        ui.text_edit_singleline(&mut self.wake_word);
                    });
                    ui.checkbox(&mut self.continuous_mode, "🔄 连续对话模式");
                    ui.add_space(10.0);

                    ui.heading("🧠 LLM 设置");
                    ui.horizontal(|ui| {
                        ui.label("模型:");
                        ui.text_edit_singleline(&mut self.llm_model);
                    });
                    ui.add_space(10.0);

                    ui.heading("🔊 TTS 设置");
                    ui.horizontal(|ui| {
                        ui.label("语音:");
                        ui.text_edit_singleline(&mut self.tts_voice);
                    });
                    ui.add_space(10.0);

                    ui.heading("🌐 语言");
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(&self.language)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.language, "中文".to_string(), "🇨🇳 中文");
                            ui.selectable_value(&mut self.language, "English".to_string(), "🇺🇸 English");
                            ui.selectable_value(&mut self.language, "日本語".to_string(), "🇯🇵 日本語");
                            ui.selectable_value(&mut self.language, "한국어".to_string(), "🇰🇷 한국어");
                        });

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        if ui.button("✅ 保存").clicked() {
                            self.show_settings = false;
                        }
                        if ui.button("❌ 取消").clicked() {
                            self.show_settings = false;
                        }
                    });
                });
        }

        // ===== 历史窗口 =====
        if self.show_history {
            egui::Window::new("📜 对话历史")
                .collapsible(false)
                .resizable(true)
                .default_width(500.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    let history_count = self.chat_history.len();
                    ui.label(format!("共 {} 条消息", history_count));
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for msg in &self.chat_history {
                            let icon = if msg.role == "user" { "👤" } else if msg.role == "system" { "ℹ️" } else { "🤖" };
                            ui.label(format!("{} [{}] {}", icon, msg.timestamp, &msg.content[..msg.content.len().min(100)]));
                        }
                    });

                    ui.add_space(10.0);
                    if ui.button("❌ 关闭").clicked() {
                        self.show_history = false;
                    }
                });
        }

        // ===== 帮助窗口 =====
        if self.show_help {
            egui::Window::new("❓ 使用帮助")
                .collapsible(false)
                .resizable(false)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.heading("🎤 语音命令");
                    ui.label("\"Hey Mini\" - 唤醒助手");
                    ui.label("\"今天天气怎么样\" - 提问");
                    ui.label("\"搜索 xxx\" - 联网搜索");
                    ui.label("\"记住 xxx\" - 存储记忆");
                    ui.label("\"5分钟后提醒我\" - 设置提醒");
                    ui.label("\"截图\" - 截屏");
                    ui.label("\"导出对话\" - 导出记录");
                    ui.add_space(10.0);

                    ui.heading("💻 系统命令");
                    ui.label("\"打开浏览器/计算器/记事本/文件夹\"");
                    ui.label("\"锁屏/关机/重启/休眠\"");
                    ui.add_space(10.0);

                    ui.heading("📋 剪贴板");
                    ui.label("\"剪贴板\" - 读取剪贴板内容");
                    ui.label("\"复制 xxx\" - 复制到剪贴板");
                    ui.add_space(10.0);

                    ui.heading("🔧 快捷键");
                    ui.label("Ctrl+Shift+V - 全局激活");
                    ui.add_space(10.0);

                    ui.separator();
                    if ui.button("❌ 关闭").clicked() {
                        self.show_help = false;
                    }
                });
        }
    }
}
