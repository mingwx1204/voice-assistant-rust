// 系统托盘功能需要额外依赖，暂用占位实现。

pub struct SystemTray;

impl Default for SystemTray {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTray {
    pub fn new() -> Self {
        tracing::info!("System tray: placeholder mode");
        Self
    }
}
