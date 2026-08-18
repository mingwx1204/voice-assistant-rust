/// agent/mod.rs — Agent 子系统
/// ==============================
/// 统一管理 Agent 人格、工具和编排。
pub mod orchestrator;
pub mod persona;
pub mod tools;

pub use orchestrator::AgentOrchestrator;
pub use persona::AgentPersona;
pub use tools::ToolRegistry;
