/// audio/playback.rs — 音频播放模块
/// ====================================
/// 基于 cpal 的跨平台音频播放，支持打断。
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 音频播放器
pub struct AudioPlayback {
    device: Device,
    config: StreamConfig,
}

impl AudioPlayback {
    /// 创建音频播放器
    pub fn new(_sample_rate: u32, _channels: u16) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No output audio device available")?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown".into());
        tracing::info!("Audio output device: {}", device_name);

        let supported = device
            .default_output_config()
            .context("No default output config")?;

        tracing::info!(
            "Output config: {} ch, {:?}, {:?}",
            supported.channels(),
            supported.sample_format(),
            supported.sample_rate()
        );

        let config = StreamConfig {
            channels: supported.channels(),
            sample_rate: supported.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self { device, config })
    }

    /// 播放 f32 音频数据，返回可打断的句柄
    pub fn play(&self, samples: &[f32]) -> Result<PlayHandle> {
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupted_clone = interrupted.clone();

        let channels = self.config.channels as usize;
        let frame_count = samples.len() / channels;
        let samples_owned = samples.to_vec();

        let stream = match self.device.default_output_config()?.sample_format() {
            SampleFormat::F32 => self.device.build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if interrupted_clone.load(Ordering::Relaxed) {
                        data.fill(0.0);
                        return;
                    }
                    for (i, frame) in data.chunks_mut(channels).enumerate() {
                        let src_idx = i * channels;
                        for (ch, sample) in frame.iter_mut().enumerate() {
                            if src_idx + ch < samples_owned.len() {
                                *sample = samples_owned[src_idx + ch];
                            } else {
                                *sample = 0.0;
                            }
                        }
                    }
                },
                |err| tracing::error!("Audio playback error: {}", err),
                None,
            )?,
            SampleFormat::I16 => self.device.build_output_stream(
                &self.config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    if interrupted_clone.load(Ordering::Relaxed) {
                        data.fill(0);
                        return;
                    }
                    for (i, frame) in data.chunks_mut(channels).enumerate() {
                        let src_idx = i * channels;
                        for (ch, sample) in frame.iter_mut().enumerate() {
                            if src_idx + ch < samples_owned.len() {
                                let s = samples_owned[src_idx + ch];
                                *sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            } else {
                                *sample = 0;
                            }
                        }
                    }
                },
                |err| tracing::error!("Audio playback error: {}", err),
                None,
            )?,
            fmt => anyhow::bail!("Unsupported output format: {:?}", fmt),
        };

        stream.play().context("Failed to start playback stream")?;

        let duration = std::time::Duration::from_secs_f32(
            frame_count as f32 / self.config.sample_rate.0 as f32,
        );

        Ok(PlayHandle {
            _stream: stream,
            interrupted,
            duration,
        })
    }

    /// 播放 WAV 数据
    pub fn play_wav(&self, wav_data: &[u8]) -> Result<PlayHandle> {
        let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_data))
            .context("Failed to parse WAV data")?;

        let spec = reader.spec();
        tracing::debug!(
            "WAV: {} Hz, {} ch, {} bit",
            spec.sample_rate,
            spec.channels,
            spec.bits_per_sample,
        );

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / (1i32 << (spec.bits_per_sample - 1)) as f32))
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to read WAV samples")?,
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to read WAV samples")?,
        };

        self.play(&samples)
    }

    /// 停止播放
    pub fn stop(&self) {
        // Stream 在 drop 时自动停止
    }
}

/// 播放句柄，drop 时停止播放
pub struct PlayHandle {
    _stream: Stream,
    interrupted: Arc<AtomicBool>,
    duration: std::time::Duration,
}

impl PlayHandle {
    /// 打断播放
    pub fn interrupt(&self) {
        self.interrupted.store(true, Ordering::Relaxed);
    }

    /// 等待播放完成
    pub fn wait(self) {
        std::thread::sleep(self.duration);
        drop(self);
    }

    /// 等待播放完成或被中断
    pub fn wait_or_interrupt(&self, check_interval: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < self.duration {
            if self.interrupted.load(Ordering::Relaxed) {
                return true;
            }
            std::thread::sleep(check_interval);
        }
        false
    }
}
