/// ui/hotkey.rs — 全局热键
/// ==========================
/// 通过轮询键盘状态实现 Ctrl+Shift+V 全局唤醒。

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 全局热键监听器
pub struct HotkeyListener {
    running: Arc<Mutex<bool>>,
}

impl HotkeyListener {
    /// 创建热键监听器，Ctrl+Shift+V 触发回调
    pub fn new(callback: impl Fn() + Send + Sync + 'static) -> Self {
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        let callback = Arc::new(callback);

        std::thread::spawn(move || {
            let mut ctrl_pressed = false;
            let mut shift_pressed = false;
            let mut v_pressed = false;

            while *running_clone.lock().unwrap() {
                // 使用 Windows API 检查按键状态
                unsafe {
                    #[cfg(target_os = "windows")]
                    {
                        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                            GetAsyncKeyState, VK_CONTROL, VK_SHIFT, VK_LCONTROL, VK_RCONTROL,
                            VK_LSHIFT, VK_RSHIFT, VK_V,
                        };

                        ctrl_pressed = (GetAsyncKeyState(VK_CONTROL.into()) as u16 & 0x8000 != 0)
                            || (GetAsyncKeyState(VK_LCONTROL.into()) as u16 & 0x8000 != 0)
                            || (GetAsyncKeyState(VK_RCONTROL.into()) as u16 & 0x8000 != 0);
                        shift_pressed = (GetAsyncKeyState(VK_SHIFT.into()) as u16 & 0x8000 != 0)
                            || (GetAsyncKeyState(VK_LSHIFT.into()) as u16 & 0x8000 != 0)
                            || (GetAsyncKeyState(VK_RSHIFT.into()) as u16 & 0x8000 != 0);
                        v_pressed = GetAsyncKeyState(VK_V.into()) as u16 & 0x8000 != 0;
                    }
                }

                if ctrl_pressed && shift_pressed && v_pressed {
                    tracing::info!("Global hotkey Ctrl+Shift+V triggered");
                    callback();
                    // 等待按键释放
                    std::thread::sleep(Duration::from_millis(500));
                }

                std::thread::sleep(Duration::from_millis(50));
            }
        });

        Self { running }
    }

    /// 停止监听
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
}
