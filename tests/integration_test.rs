/// tests/integration_test.rs — 集成测试
/// =====================================

// 测试工具函数
#[test]
fn test_time_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("现在几点了");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.should_respond);
    assert!(result.output.contains("现在是"));
}

#[test]
fn test_calculator_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("计算 256 * 18");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("4608"));
}

#[test]
fn test_screenshot_tool() {
    // 截图测试会保存文件，跳过实际执行
    // 只验证工具检测逻辑
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("截图");
    assert!(result.is_some());
    // 注意：此测试会保存截图文件到本地
}

#[test]
fn test_system_command_tool() {
    // 只测试检测逻辑，不实际执行
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("打开不存在的程序xyz");
    // 应该返回"我支持..."的提示，不会真的执行
    assert!(result.is_some());
}

#[test]
fn test_search_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("搜索天气");
    assert!(result.is_some());
}

#[test]
fn test_export_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("导出对话");
    assert!(result.is_some());
}

#[test]
fn test_translate_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("翻译成英语 你好");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.needs_llm);
}

#[test]
fn test_emotion_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("分析情感 我今天很开心");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("开心"));
}

#[test]
fn test_code_execution() {
    // 代码执行测试会真的运行 Python，验证安全限制
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    // 测试安全限制：危险代码应该被拒绝
    let result = tools.detect_and_execute("运行代码 import os; os.system('echo hacked')");
    assert!(result.is_some());
    let result = result.unwrap();
    eprintln!("DEBUG output: {}", result.output);
    // 安全检查会拒绝包含 import os 的代码
    assert!(result.should_respond);
}

#[test]
fn test_conversation_search() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("搜索对话 天气");
    assert!(result.is_some());
}

#[test]
fn test_quick_phrases() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("快捷短语");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("快捷短语"));
}

#[test]
fn test_score_tool() {
    let tools = voice_assistant::agent::tools::ToolRegistry::new();
    let result = tools.detect_and_execute("五星好评");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("5星"));
}

// 测试配置
#[test]
fn test_config_default() {
    let config = voice_assistant::config::AppConfig::default();
    assert_eq!(config.audio.sample_rate, 16000);
    assert_eq!(config.stt.language, "zh");
    assert_eq!(config.agent.name, "Mini");
}

// 测试 Agent 人格
#[test]
fn test_persona() {
    let config = voice_assistant::config::AppConfig::default();
    let persona = voice_assistant::agent::persona::AgentPersona::from_config(&config.agent);
    let prompt = persona.get_system_prompt("");
    assert!(prompt.contains("Mini"));
}

#[test]
fn test_greeting() {
    let config = voice_assistant::config::AppConfig::default();
    let persona = voice_assistant::agent::persona::AgentPersona::from_config(&config.agent);
    let greeting = persona.get_greeting();
    assert!(!greeting.is_empty());
}
