# Models Directory

This directory should contain the following model files:

## Required Models

### 1. Silero VAD (Voice Activity Detection)
- File: `silero_vad.onnx`
- Source: https://github.com/snakers4/silero-vad
- Size: ~2MB
- Description: Neural network VAD model for detecting speech in audio

### 2. Whisper (Speech-to-Text)
- File: `ggml-base.bin` (or other size)
- Source: https://huggingface.co/ggerganov/whisper.cpp
- Size: ~142MB (base), ~466MB (small), ~1.5GB (medium)
- Description: OpenAI Whisper model in GGML format for speech recognition

### 3. Piper TTS (Text-to-Speech)
- Directory: `tts-models/` or specify in config
- Source: https://github.com/rhasspy/piper
- Example models:
  - `zh_CN-huayan-medium` (Chinese female voice)
  - `zh_CN-huayan-medium.onnx` + `config.json`
- Description: Neural network TTS model for speech synthesis

## Download Instructions

```bash
# Silero VAD
# Download from: https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx

# Whisper
# Download from: https://huggingface.co/ggerganov/whisper.cpp/tree/main
# Example: ggml-base.bin

# Piper TTS
# Download from: https://huggingface.co/rhasspy/piper-voices/tree/main/zh_CN/huayan/medium
```

## Configuration

Model paths can be configured in `config.json`:

```json
{
  "stt": {
    "model_path": "models/ggml-base.bin"
  },
  "tts": {
    "model_dir": "models/tts-models"
  }
}
```
