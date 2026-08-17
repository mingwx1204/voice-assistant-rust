# 🤖 Mini 语音助手 v0.1.0 (Rust)

> 纯本地运行、带 Agent 的 AI 语音对话助手 — Rust 重写版

## ✨ 功能特性

| 功能 | 说明 |
|------|------|
| 🎤 唤醒词对话 | "Hey Mini" 唤醒，无需按键 |
| 🗣️ 语音识别 | whisper-rs 本地识别，CUDA 加速 |
| 🧠 大模型推理 | llama.cpp 通过 OpenAI 兼容 API |
| 🔊 语音合成 | Piper 本地神经网络 TTS |
| 🧮 VAD 检测 | Silero VAD 神经网络语音检测 |
| 💾 记忆系统 | SQLite + FTS5 混合检索 |
| ⏰ 提醒功能 | "5分钟后提醒我喝水" |
| 🧮 工具调用 | 时间查询、数学计算 |
| 🖥️ 图形界面 | egui 即时模式 GUI |
| ⚡ 播放打断 | 说话即可打断当前播放 |

## 📋 硬件要求

- **GPU**: NVIDIA RTX 2060 6GB 或更高 (可选)
- **RAM**: 8GB+
- **Rust**: 1.75+
- **系统**: Windows / Linux / macOS

## 🚀 快速开始

### 1. 安装 Rust

```bash
# Windows
winget install Rustlang.Rustup

# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 下载模型

```bash
# 进入模型目录
cd models

# 下载 Silero VAD (~2MB)
# 从 https://github.com/snakers4/silero-vad 下载 silero_vad.onnx

# 下载 Whisper base (~142MB)
# 从 https://huggingface.co/ggerganov/whisper.cpp 下载 ggml-base.bin

# 下载 Piper TTS 模型
# 从 https://github.com/rhasspy/piper 下载中文模型
```

### 3. 启动 llama-server

```bash
D:\models\llama.cpp-mtmd-grounders\build-cuda\bin\llama-server.exe ^
  -m D:\models\minicpm-v-4.6-gguf\MiniCPM-V-4_6-Q4_K_M.gguf ^
  -c 4096 --host 127.0.0.1 --port 8080 -ngl 999
```

### 4. 编译并运行

```bash
# 编译
cargo build --release

# 运行
cargo run --release
```

## 📁 项目结构

```
voice-assistant-rust/
├── Cargo.toml              # Rust 依赖配置
├── README.md               # 本文档
├── models/                 # 模型文件目录
│   ├── silero_vad.onnx     #   Silero VAD 模型
│   ├── ggml-base.bin       #   Whisper 模型
│   └── tts-models/         #   Piper TTS 模型
│
└── src/
    ├── main.rs             # 主入口
    ├── config.rs           # 配置系统
    │
    ├── audio/              # 音频子系统 (cpal)
    │   ├── mod.rs
    │   ├── capture.rs      #   音频采集
    │   └── playback.rs     #   音频播放
    │
    ├── stt/                # 语音识别
    │   ├── mod.rs
    │   ├── vad.rs          #   Silero VAD
    │   └── whisper.rs      #   whisper-rs
    │
    ├── tts/                # 语音合成
    │   ├── mod.rs
    │   └── piper.rs        #   Piper TTS
    │
    ├── llm/                # LLM 推理
    │   ├── mod.rs
    │   └── rig_client.rs   #   rig 框架
    │
    ├── agent/              # Agent 系统
    │   ├── mod.rs
    │   ├── persona.rs      #   人格设定
    │   ├── tools.rs        #   工具集
    │   └── orchestrator.rs #   编排器
    │
    ├── memory/             # 记忆系统
    │   ├── mod.rs
    │   └── database.rs     #   SQLite 数据库
    │
    └── ui/                 # 用户界面
        └── mod.rs          #   egui GUI
```

## ⚙️ 技术栈

| 模块 | 技术 | 说明 |
|------|------|------|
| **GUI** | egui (eframe) | 即时模式 GUI，轻量高效 |
| **音频** | cpal | 纯 Rust 跨平台音频 I/O |
| **VAD** | silero-vad | 神经网络语音活动检测 |
| **STT** | whisper-rs | whisper.cpp Rust 绑定 |
| **TTS** | piper | 本地神经网络语音合成 |
| **LLM** | rig | Rust LLM agent 框架 |
| **数据库** | rusqlite | SQLite 纯 Rust 绑定 |
| **异步** | tokio | Rust 异步运行时 |

## 🔧 配置说明

配置文件 `config.json`（首次运行自动创建）：

```json
{
  "audio": {
    "sample_rate": 16000,
    "channels": 1,
    "record_duration_secs": 5.0
  },
  "stt": {
    "model_path": "models/ggml-base.bin",
    "language": "zh",
    "beam_size": 5
  },
  "llm": {
    "base_url": "http://127.0.0.1:8080/v1",
    "model": "minicpm-v",
    "max_tokens": 512
  },
  "tts": {
    "model_dir": "models/tts-models",
    "length_scale": 1.0
  }
}
```

## 📝 使用示例

```
🎤 说 "Hey Mini" 唤醒
💬 "今天天气怎么样？"
🤖 "今天天气不错，适合出门。"

🎤 再次唤醒
💬 "我之前说过我喜欢什么咖啡？"
🤖 "你之前说过你喜欢冰美式咖啡哦。"

🎤 再次唤醒
💬 "5分钟后提醒我喝水"
🤖 "好的，已设置提醒：提醒事项（5分钟后）"

🎤 再次唤醒
💬 "256乘18等于多少"
🤖 "256 * 18 = 4608"
```

## 🔄 从 Python 版迁移

本项目是 [minicpm-voice-assistant](../minicpm-voice-assistant/) 的 Rust 重写版。

### 主要改进

1. **性能**: Rust 原生性能，无 Python GIL 限制
2. **内存**: 更低的内存占用，无 Python 运行时开销
3. **部署**: 单一二进制文件，无 Python 环境依赖
4. **VAD**: Silero VAD 替代能量阈值检测，精度大幅提升
5. **TTS**: Piper 本地 TTS 替代 Edge TTS 云端服务
6. **GUI**: egui 图形界面替代控制台

### 对应关系

| Python 版 | Rust 版 |
|-----------|---------|
| sounddevice | cpal |
| faster-whisper | whisper-rs |
| edge-tts | piper |
| OpenAI SDK | rig |
| sentence-transformers | (待实现) |
| sqlite3 | rusqlite |
| pygame | cpal (播放) |

## 📄 License

MIT License
