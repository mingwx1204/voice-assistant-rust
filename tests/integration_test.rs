/// tests/integration_test.rs — 集成测试
/// =====================================

use std::sync::{Arc, Mutex};

// ===== 工具测试 =====
#[test]
fn test_time_tool() {
    // 测试时间工具
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("现在几点了");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.should_respond);
    assert!(result.output.contains("现在是"));
}

#[test]
fn test_calculator_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("计算 256 * 18");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("4608"));
}

#[test]
fn test_memory_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("记住我喜欢冰美式咖啡");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("记住了"));
}

#[test]
fn test_reminder_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("5分钟后提醒我喝水");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("提醒"));
}

#[test]
fn test_translate_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("翻译成英语 你好世界");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.needs_llm); // 翻译需要 LLM
}

#[test]
fn test_system_command_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("打开计算器");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("计算器"));
}

#[test]
fn test_clipboard_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("剪贴板");
    assert!(result.is_some()); // 应该返回剪贴板内容或空
}

#[test]
fn test_screenshot_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("截图");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("截图"));
}

#[test]
fn test_search_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("搜索今天的天气");
    assert!(result.is_some());
    // 搜索可能成功也可能失败（网络问题），但应该有响应
}

#[test]
fn test_export_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("导出对话");
    assert!(result.is_some());
}

#[test]
fn test_quick_phrases() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("快捷短语");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("快捷短语"));
}

#[test]
fn test_score_tool() {
    let tools = voice_assistant::agent::ToolRegistry::new();
    let result = tools.detect_and_execute("五星好评");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("5星"));
}

// ===== 记忆数据库测试 =====
#[test]
fn test_memory_database() {
    let db = voice_assistant::memory::MemoryDatabase::new(std::path::Path::new(":memory:")).unwrap();

    // 保存记忆
    let id = db.save_memory("我喜欢冰美式", "preference", Some("user"), None, 0.5).unwrap();
    assert!(id > 0);

    // 搜索记忆
    let results = db.search_memories_fts("咖啡", 10).unwrap();
    assert!(!results.is_empty());

    // 获取统计
    let stats = db.get_stats().unwrap();
    assert!(stats.memories > 0);
}

#[test]
fn test_reminder_database() {
    let db = voice_assistant::memory::MemoryDatabase::new(std::path::Path::new(":memory:")).unwrap();

    let remind_at = chrono::Local::now().naive_local() + chrono::Duration::minutes(5);
    let id = db.create_reminder(&remind_at, "喝水").unwrap();
    assert!(id > 0);

    let reminders = db.get_pending_reminders().unwrap();
    assert!(!reminders.is_empty());

    db.fire_reminder(id).unwrap();
    let reminders = db.get_pending_reminders().unwrap();
    assert!(reminders.is_empty());
}

// ===== 配置测试 =====
#[test]
fn test_config_default() {
    let config = voice_assistant::config::AppConfig::default();
    assert_eq!(config.audio.sample_rate, 16000);
    assert_eq!(config.stt.language, "zh");
    assert_eq!(config.agent.name, "Mini");
}

// ===== Agent 测试 =====
#[test]
fn test_persona() {
    let config = voice_assistant::config::AppConfig::default();
    let persona = voice_assistant::agent::AgentPersona::from_config(&config.agent);
    let prompt = persona.get_system_prompt("");
    assert!(prompt.contains("Mini"));
    assert!(prompt.contains("核心能力"));
}

#[test]
fn test_greeting() {
    let config = voice_assistant::config::AppConfig::default();
    let persona = voice_assistant::agent::AgentPersona::from_config(&config.agent);
    let greeting = persona.get_greeting();
    assert!(!greeting.is_empty());
}
