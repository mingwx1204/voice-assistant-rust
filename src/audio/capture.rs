/// audio/capture.rs — 音频采集模块
/// ==================================
/// 基于 cpal 的跨平台音频采集。

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// 音频采集器
pub struct AudioCapture {
    device: Device,
    config: StreamConfig,
    sample_rate: u32,
    /// 实时音量 (0.0-1.0)
    pub volume: Arc<Mutex<f32>>,
}

impl AudioCapture {
    /// 创建音频采集器
    pub fn new(_sample_rate: u32, _channels: u16, _block_size: usize) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input audio device available")?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown".into());
        tracing::info!("Audio input device: {}", device_name);

        // 使用设备默认配置
        let supported = device
            .default_input_config()
            .context("No default input config")?;

        tracing::info!(
            "Supported config: {} ch, {:?}, {:?}",
            supported.channels(),
            supported.sample_format(),
            supported.sample_rate()
        );

        // 使用设备支持的采样率和声道数
        let actual_sample_rate = supported.sample_rate().0;
        let actual_channels = supported.channels();

        let config = StreamConfig {
            channels: actual_channels,
            sample_rate: supported.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self {
            device,
            config,
            sample_rate: actual_sample_rate,
            volume: Arc::new(Mutex::new(0.0)),
        })
    }

    /// 录制指定时长的音频，返回 f32 采样数据 (归一化到 -1.0 ~ 1.0)
    pub fn record_blocking(&mut self, duration_secs: f32) -> Result<Vec<f32>> {
        let total_frames = (self.sample_rate as f32 * duration_secs) as usize;
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(total_frames)));
        let buffer_clone = buffer.clone();
        let volume_clone = self.volume.clone();

        let (tx, rx) = mpsc::sync_channel::<()>(1);

        let sample_format = self.device
            .default_input_config()
            .context("No input config")?
            .sample_format();

        let stream = match sample_format {
            SampleFormat::F32 => self.device.build_input_stream(
                &self.config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // 计算实时音量
                    let sum: f32 = data.iter().map(|s| s * s).sum();
                    let rms = (sum / data.len() as f32).sqrt();
                    *volume_clone.lock().unwrap() = (rms * 3.0).min(1.0);

                    let mut buf = buffer_clone.lock().unwrap();
                    buf.extend_from_slice(data);
                    if buf.len() >= total_frames {
                        let _ = tx.send(());
                    }
                },
                |err| tracing::error!("Audio capture error: {}", err),
                None,
            )?,
            SampleFormat::I16 => self.device.build_input_stream(
                &self.config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let sum: f64 = data.iter().map(|&s| (s as f64 / i16::MAX as f64).powi(2)).sum();
                    let rms = (sum / data.len() as f64).sqrt() as f32;
                    *volume_clone.lock().unwrap() = (rms * 3.0).min(1.0);

                    let mut buf = buffer_clone.lock().unwrap();
                    for &sample in data {
                        buf.push(sample as f32 / i16::MAX as f32);
                    }
                    if buf.len() >= total_frames {
                        let _ = tx.send(());
                    }
                },
                |err| tracing::error!("Audio capture error: {}", err),
                None,
            )?,
            SampleFormat::U16 => self.device.build_input_stream(
                &self.config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut buf = buffer_clone.lock().unwrap();
                    for &sample in data {
                        buf.push((sample as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    }
                    if buf.len() >= total_frames {
                        let _ = tx.send(());
                    }
                },
                |err| tracing::error!("Audio capture error: {}", err),
                None,
            )?,
            fmt => anyhow::bail!("Unsupported sample format: {:?}", fmt),
        };

        stream.play().context("Failed to start audio stream")?;

        // 等待录制完成或超时
        let timeout = std::time::Duration::from_secs_f32(duration_secs + 0.5);
        let _ = rx.recv_timeout(timeout);

        // 停止流
        drop(stream);

        let mut buf = buffer.lock().unwrap();
        buf.truncate(total_frames);
        let samples = std::mem::take(&mut *buf);

        tracing::info!("Recorded {} frames ({:.1}s)", samples.len(), duration_secs);
        Ok(samples)
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
