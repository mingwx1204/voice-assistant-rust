/// main.rs — Voice Assistant 入口 v0.3.0
/// =========================================
/// 完整版：GUI + 系统托盘 + 全局热键 + 全功能

mod agent;
mod audio;
mod config;
mod llm;
mod memory;
mod stt;
mod tts;
mod ui;

use anyhow::Result;
use eframe::egui;
use std::sync::{Arc, Mutex};

use config::AppConfig;
use ui::VoiceAssistantApp;

fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,voice_assistant=debug".into()),
        )
        .init();

    tracing::info!("Starting Mini Voice Assistant v0.3.0 (Rust)");

    // 加载配置
    let config = AppConfig::load();

    // 列出音频设备
    if let Err(e) = audio::list_devices() {
        tracing::warn!("Failed to list audio devices: {}", e);
    }

    // 创建共享的 UI 状态
    let app_state = Arc::new(Mutex::new(VoiceAssistantApp::new()));

    // 创建编排器
    let mut orchestrator = agent::AgentOrchestrator::new(
        config.clone(),
        app_state.clone(),
    );

    // 初始化组件
    if let Err(e) = orchestrator.initialize() {
        tracing::error!("Initialization failed: {}", e);
        eprintln!("Initialization failed: {}", e);
    }

    // 启动编排器线程
    let orchestrator_handle = Arc::new(Mutex::new(orchestrator));
    let orchestrator_clone = orchestrator_handle.clone();

    let _orchestrator_thread = std::thread::spawn(move || {
        let mut orch = orchestrator_clone.lock().unwrap();
        if let Err(e) = orch.run() {
            tracing::error!("Orchestrator error: {}", e);
        }
    });

    // 启动全局热键 (Ctrl+Shift+V)
    let hotkey_state = app_state.clone();
    let _hotkey = ui::hotkey::HotkeyListener::new(move || {
        let mut app = hotkey_state.lock().unwrap();
        app.add_message("system", "🎤 热键唤醒 — 请说话...");
        app.set_state(ui::AppState::Listening);
    });

    // 启动 GUI
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Mini 语音助手 v0.3.0"),
        ..Default::default()
    };

    let app_state_clone = app_state.clone();

    eframe::run_native(
        "Mini Voice Assistant",
        native_options,
        Box::new(move |cc| {
            setup_fonts(&cc.egui_ctx);
            let app = VoiceAssistantApp::new();
            *app_state_clone.lock().unwrap() = app;
            Ok(Box::new(SharedAppWrapper {
                app_state: app_state_clone,
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;

    Ok(())
}

/// 设置中文字体
fn setup_fonts(ctx: &egui::Context) {
    use egui::FontFamily;
    let mut fonts = egui::FontDefinitions::default();

    let chinese_font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
    ];

    let mut loaded = false;
    for font_path in &chinese_font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            tracing::info!("Loading Chinese font: {}", font_path);
            fonts.font_data.insert("chinese".to_owned(), egui::FontData::from_owned(font_data).into());
            fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "chinese".to_owned());
            fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "chinese".to_owned());
            loaded = true;
            break;
        }
    }

    if loaded { tracing::info!("Chinese font loaded"); }
    else { tracing::warn!("No Chinese font found"); }

    ctx.set_fonts(fonts);
}

/// GUI 包装器
struct SharedAppWrapper {
    app_state: Arc<Mutex<VoiceAssistantApp>>,
}

impl eframe::App for SharedAppWrapper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut app = self.app_state.lock().unwrap();
        app.update(ctx, _frame);
    }
}
