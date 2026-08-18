/// ui/tray.rs — 系统托盘（占位）
/// ================================
/// 系统托盘功能需要额外依赖，暂用占位实现。

pub struct SystemTray;

impl SystemTray {
    pub fn new() -> Self {
        tracing::info!("System tray: placeholder mode");
        Self
    }
}
