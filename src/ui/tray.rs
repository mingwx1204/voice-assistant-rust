/// ui/tray.rs — 系统托盘
/// ========================
/// 最小化到系统托盘，右键菜单操作。

use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};

pub struct SystemTray {
    #[allow(dead_code)]
    tray: tray_icon::TrayIcon,
}

impl SystemTray {
    pub fn new() -> Self {
        let menu = Menu::new();
        let _show = MenuItem::new("显示窗口", true, None);
        let _sep = muda::PredefinedMenuItem::separator();
        let _quit = MenuItem::new("退出", true, None);

        menu.append(&_show).unwrap();
        menu.append(&_sep).unwrap();
        menu.append(&_quit).unwrap();

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Mini 语音助手 v0.3.0")
            .build()
            .unwrap();

        tracing::info!("System tray created");
        Self { tray }
    }
}
