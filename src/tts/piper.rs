/// tts/piper.rs — Piper TTS 语音合成
/// ====================================
/// 使用 Python piper 通过 subprocess 合成语音。

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Piper TTS 引擎
pub struct PiperTts {
    model_path: PathBuf,
    #[allow(dead_code)]
    sample_rate: u32,
}

impl PiperTts {
    /// 创建 Piper TTS 实例
    pub fn new(model_dir: &Path, _speaker_id: Option<u32>, _length_scale: f32) -> Result<Self> {
        // 查找 ONNX 模型
        let mut model_path = None;
        if model_dir.exists() {
            for entry in std::fs::read_dir(model_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("onnx") && !path.to_string_lossy().contains(".json") {
                    model_path = Some(path);
                    break;
                }
            }
        }

        let model_path = model_path.unwrap_or_else(|| {
            tracing::warn!("Piper model not found in {:?}, using placeholder", model_dir);
            PathBuf::new()
        });

        let sample_rate = 22050; // Piper 默认采样率

        tracing::info!("Piper TTS: model={:?}, sample_rate={}", model_path, sample_rate);

        Ok(Self { model_path, sample_rate })
    }

    /// 合成语音 — 返回 WAV 字节数据
    pub fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        if self.model_path.as_os_str().is_empty() {
            // 占位模式：返回静音
            return Ok(self.generate_silence(0.5));
        }

        // 尝试使用 Python piper
        match self.synthesize_via_python(text) {
            Ok(wav) => return Ok(wav),
            Err(e) => {
                tracing::warn!("Python piper failed: {}, trying fallback", e);
            }
        }

        // 降级：返回静音
        Ok(self.generate_silence(0.5))
    }

    /// 通过 Python piper 合成
    fn synthesize_via_python(&self, text: &str) -> Result<Vec<u8>> {
        let output_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("voice-assistant").join("tts_output");
        std::fs::create_dir_all(&output_dir)?;

        let output_path = output_dir.join("output.wav");

        // 使用 Python 调用 piper
        let python_script = format!(
            r#"
import sys
try:
    from piper import PiperVoice
    import wave
    import io
    
    voice = PiperVoice.load(r"{}")
    wav_buffer = io.BytesIO()
    with wave.open(wav_buffer, 'wb') as wav_file:
        voice.synthesize("{}", wav_file)
    
    with open(r"{}", 'wb') as f:
        f.write(wav_buffer.getvalue())
    
    print("OK")
except ImportError:
    # piper 未安装，使用 pyttsx3 作为备选
    try:
        import pyttsx3
        engine = pyttsx3.init()
        engine.setProperty('rate', 150)
        engine.save_to_file("{}", r"{}")
        engine.runAndWait()
        print("OK")
    except:
        print("NO_PIPER")
except Exception as e:
    print(f"ERROR: {{e}}")
"#,
            self.model_path.display(),
            text.replace('"', "\\\""),
            output_path.display(),
            text,
            output_path.display()
        );

        let output = Command::new("python")
            .args(["-c", &python_script])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim() == "OK" && output_path.exists() {
            let wav_data = std::fs::read(&output_path)?;
            let _ = std::fs::remove_file(&output_path);
            Ok(wav_data)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Python piper failed: {} {}", stdout, stderr)
        }
    }

    /// 生成静音 WAV
    fn generate_silence(&self, duration_secs: f32) -> Vec<u8> {
        let num_samples = (self.sample_rate as f32 * duration_secs) as usize;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut buf, spec).unwrap();
            for _ in 0..num_samples {
                writer.write_sample(0i16).unwrap();
            }
            writer.finalize().unwrap();
        }
        buf.into_inner()
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
