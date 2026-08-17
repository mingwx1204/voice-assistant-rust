/// llm/mod.rs — LLM 推理子系统
/// =================================
/// 统一管理 LLM 客户端。

pub mod rig_client;

pub use rig_client::{ChatMessage, LlmClient};
