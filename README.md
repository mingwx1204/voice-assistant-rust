# 🤖 Mini 语音助手 v0.3.0 (Rust)

> 纯本地运行、带 Agent 的 AI 语音对话助手 — 30 项功能全集成

## ✨ 功能特性

### 🎙️ 语音交互
- 🔄 **多轮连续对话** — 回复后继续监听 10 秒
- ⚡ **流式响应** — LLM 边生成边显示
- 🧠 **记忆自动提炼** — 每 3 轮对话自动提取
- 🌐 **多语言自动切换** — Whisper 检测输入语言
- 🎤 **唤醒词** — "Hey Mini" 或自定义

### 🔧 工具集 (14 个)
| 工具 | 触发方式 | 说明 |
|------|---------|------|
| ⏰ 时间 | "几点了" | 当前日期时间 |
| 🧮 计算 | "256乘18" | 数学表达式 |
| 🔍 搜索 | "搜索xxx" | DuckDuckGo 联网搜索 |
| 📸 截图 | "截图" | Windows API 截屏 |
| 💻 系统 | "打开浏览器" | 白名单安全执行 |
| 📋 剪贴板 | "剪贴板" | 读取/复制 |
| 🔄 翻译 | "翻译成英语" | LLM 翻译 |
| 🔔 通知 | "通知xxx" | Windows Toast |
| ⏰ 定时 | "每隔30分钟" | 定时任务 |
| 📝 快捷 | "快捷短语" | 预设命令 |
| ⭐ 评分 | "五星好评" | 对话评分 |
| 💾 导出 | "导出对话" | Markdown 格式 |
| ⏰ 提醒 | "5分钟后提醒我" | 定时提醒 |
| 🧠 记忆 | "记住xxx" | 长期记忆 |

### 🖥️ 界面
- 📊 实时音量/波形显示
- ⚙️ 设置窗口 (配置持久化)
- 📜 对话历史窗口
- ❓ 帮助窗口
- 🎯 快捷按钮面板
- 💻 系统命令菜单
- 🔗 系统托盘 (后台运行)
- ⌨️ 全局热键 (Ctrl+Shift+V)

### 📚 记忆系统
- 💾 SQLite + FTS5 全文检索
- 📚 RAG 知识库 (加载本地 txt/md)
- 🧠 LLM 记忆提炼
- 🔄 对话历史持久化

## 📋 硬件要求

- **GPU**: NVIDIA RTX 2060 6GB 或更高 (可选)
- **RAM**: 8GB+
- **系统**: Windows 10/11

## 🚀 快速开始

### 1. 安装 Rust

```bash
winget install Rustlang.Rustup
```

### 2. 克隆项目

```bash
git clone https://github.com/mingwx1204/voice-assistant-rust.git
cd voice-assistant-rust
```

### 3. 编译

```powershell
# 方式 1: 使用构建脚本
.\build.ps1 -Release

# 方式 2: 直接编译
cargo build --release
```

### 4. 运行

```powershell
# 方式 1: 直接运行
cargo run --release

# 方式 2: 安装到系统
.\install.ps1
```

### 5. 前置条件

- **Whisper 模型**: 放在 `models/ggml-base.bin`
- **llama-server**: 需要先启动 LLM 推理服务

## 📁 项目结构

```
voice-assistant-rust/
├── src/
│   ├── main.rs              # 入口 + egui GUI
│   ├── config.rs            # 配置系统
│   ├── agent/               # Agent 系统
│   │   ├── persona.rs       #   人格设定
│   │   ├── tools.rs         #   工具集 (14个)
│   │   └── orchestrator.rs  #   编排器
│   ├── audio/               # 音频 (cpal)
│   ├── stt/                 # 语音识别 (whisper-rs)
│   ├── tts/                 # 语音合成 (piper)
│   ├── llm/                 # LLM 客户端
│   ├── memory/              # 记忆系统
│   │   ├── database.rs      #   SQLite
│   │   └── rag.rs           #   RAG 知识库
│   └── ui/                  # GUI
│       ├── mod.rs           #   egui 界面
│       ├── tray.rs          #   系统托盘
│       └── hotkey.rs        #   全局热键
├── tests/                   # 测试
├── .github/workflows/       # CI/CD
├── build.ps1                # 构建脚本
├── install.ps1              # 安装脚本
└── Cargo.toml
```

## ⚙️ 配置说明

配置文件 `config.json`（首次运行自动创建）：

```json
{
  "audio": {
    "sample_rate": 16000,
    "channels": 1,
    "record_duration_secs": 5.0,
    "continuous_timeout_secs": 10.0
  },
  "stt": {
    "model_path": "models/ggml-base.bin",
    "language": "zh",
    "beam_size": 5
  },
  "llm": {
    "base_url": "http://127.0.0.1:8080/v1",
    "model": "minicpm-v",
    "max_tokens": 512,
    "temperature": 0.7
  },
  "tts": {
    "model_dir": "models/tts-models",
    "length_scale": 1.0
  },
  "memory": {
    "extract_interval": 3,
    "top_k": 5
  }
}
```

## 🔧 开发

### 构建命令

```powershell
# 检查代码
cargo check

# 运行测试
cargo test

# 编译 (debug)
cargo build

# 编译 (release)
cargo build --release

# 运行
cargo run --release
```

### 添加知识库

把 `.txt` 或 `.md` 文件放到知识库目录：

```
Windows: %LOCALAPPDATA%\voice-assistant\knowledge\
Linux:   ~/.local/share/voice-assistant/knowledge/
```

### 工具扩展

在 `src/agent/tools.rs` 中添加新工具：

```rust
fn my_new_tool(&self, text: &str) -> ToolResult {
    // 实现逻辑
    ToolResult::simple("结果")
}

// 在 detect_and_execute 中添加触发条件
if text.contains("我的工具") {
    return Some(self.my_new_tool(text));
}
```

## 📊 性能

| 指标 | 值 |
|------|-----|
| 编译时间 | ~1s (增量) |
| 内存占用 | ~50MB (空闲) |
| STT 延迟 | ~1s (base 模型) |
| LLM 延迟 | ~2s (取决于模型) |
| TTS 延迟 | ~0.5s |

## 🐛 常见问题

### Q: 没有声音？
A: 检查系统音频设备，确保麦克风和扬声器正常。

### Q: LLM 不响应？
A: 确保 llama-server 正在运行：
```bash
curl http://127.0.0.1:8080/v1/models
```

### Q: Whisper 模型找不到？
A: 下载模型到 `models/ggml-base.bin`：
```bash
# 使用 Python 下载
python -c "import urllib.request; urllib.request.urlretrieve('https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-base.bin', 'models/ggml-base.bin')"
```

### Q: 全局热键不工作？
A: 确保没有其他程序占用 Ctrl+Shift+V。

## 📄 License

MIT License

## 🙏 致谢

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — 语音识别
- [llama.cpp](https://github.com/ggerganov/llama.cpp) — LLM 推理
- [egui](https://github.com/emilk/egui) — GUI 框架
- [cpal](https://github.com/RustAudio/cpal) — 音频 I/O
- [rusqlite](https://github.com/rusqlite/rusqlite) — SQLite
