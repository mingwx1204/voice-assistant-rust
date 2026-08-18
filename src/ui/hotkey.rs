/// ui/hotkey.rs — 全局热键（占位）
/// ==================================
/// 全局热键功能需要 windows-sys，暂用占位实现。
use std::sync::{Arc, Mutex};

pub struct HotkeyListener {
    running: Arc<Mutex<bool>>,
}

impl HotkeyListener {
    pub fn new(_callback: impl Fn() + Send + Sync + 'static) -> Self {
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();

        // 占位：仅打印日志
        std::thread::spawn(move || {
            tracing::info!("Global hotkey listener started (placeholder)");
            while *running_clone.lock().unwrap() {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Self { running }
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
}
