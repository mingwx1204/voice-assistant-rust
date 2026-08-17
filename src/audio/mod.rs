/// audio/mod.rs — 音频子系统
/// ============================
/// 统一管理音频采集和播放。

pub mod capture;
pub mod playback;

pub use capture::AudioCapture;
pub use playback::{AudioPlayback, PlayHandle};

use cpal::traits::{DeviceTrait, HostTrait};
use anyhow::Result;

/// 列出所有音频设备
pub fn list_devices() -> Result<()> {
    let host = cpal::default_host();

    println!("\n=== Audio Devices ===");

    println!("\n[Input Devices]");
    if let Some(device) = host.default_input_device() {
        let name = device.name().unwrap_or_else(|_| "Unknown".into());
        if let Ok(config) = device.default_input_config() {
            println!(
                "  * {} ({} ch, {} Hz, {:?}) [DEFAULT]",
                name,
                config.channels(),
                config.sample_rate().0,
                config.sample_format()
            );
        }
    }

    println!("\n[Output Devices]");
    if let Some(device) = host.default_output_device() {
        let name = device.name().unwrap_or_else(|_| "Unknown".into());
        if let Ok(config) = device.default_output_config() {
            println!(
                "  * {} ({} ch, {} Hz, {:?}) [DEFAULT]",
                name,
                config.channels(),
                config.sample_rate().0,
                config.sample_format()
            );
        }
    }

    println!();
    Ok(())
}
