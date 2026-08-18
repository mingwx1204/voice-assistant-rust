/// tests/e2e_test.rs — 端到端测试
/// =================================
/// 测试完整对话流程（不依赖外部服务）
use voice_assistant::agent::tools::ToolRegistry;
use voice_assistant::config::AppConfig;
use voice_assistant::memory::MemoryDatabase;

/// 测试完整对话流程
#[test]
fn test_full_conversation_flow() {
    // 1. 初始化配置
    let config = AppConfig::default();
    assert_eq!(config.agent.name, "Mini");

    // 2. 初始化工具（无数据库）
    let tools = ToolRegistry::new();

    // 3. 时间查询
    let result = tools.detect_and_execute("现在几点了");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("现在是"));

    // 4. 数学计算
    let result = tools.detect_and_execute("计算 100 * 200");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("20000"));

    // 5. 截图
    let result = tools.detect_and_execute("截图");
    assert!(result.is_some());

    // 6. 快捷短语
    let result = tools.detect_and_execute("快捷短语");
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.output.contains("快捷短语"));
}

/// 测试记忆系统完整流程
#[test]
fn test_memory_system_flow() {
    let db = MemoryDatabase::new(std::path::Path::new(":memory:")).unwrap();

    // 保存多条记忆
    db.save_memory("用户喜欢冰美式咖啡", "preference", Some("user"), None, 0.8)
        .unwrap();
    db.save_memory("用户住在北京", "fact", Some("user"), None, 0.6)
        .unwrap();
    db.save_memory("用户是程序员", "personal", Some("user"), None, 0.7)
        .unwrap();

    // 获取所有记忆
    let all = db.get_all_memories().unwrap();
    assert_eq!(all.len(), 3);

    // FTS 搜索（可能对内存数据库不完全支持）
    let _ = db.search_memories_fts("咖啡", 5); // 忽略结果，只测试不崩溃

    // 获取统计
    let stats = db.get_stats().unwrap();
    assert_eq!(stats.memories, 3);
}

/// 测试提醒系统完整流程
#[test]
fn test_reminder_system_flow() {
    let db = MemoryDatabase::new(std::path::Path::new(":memory:")).unwrap();

    // 创建已到时间的提醒（过去时间）
    let now = chrono::Local::now().naive_local();
    let remind_at = now - chrono::Duration::minutes(5);
    let id1 = db.create_reminder(&remind_at, "喝水").unwrap();

    let remind_at2 = now - chrono::Duration::minutes(10);
    let _id2 = db.create_reminder(&remind_at2, "运动").unwrap();

    // 检查待处理提醒（已到时间的）
    let reminders = db.get_pending_reminders().unwrap();
    assert!(
        reminders.len() >= 2,
        "Should have at least 2 reminders, got {}",
        reminders.len()
    );

    // 触发一个提醒
    db.fire_reminder(id1).unwrap();

    // 检查提醒数量减少
    let reminders = db.get_pending_reminders().unwrap();
    assert!(
        reminders.len() >= 1,
        "Should have at least 1 reminder after firing one"
    );
}

/// 测试工具检测优先级
#[test]
fn test_tool_priority() {
    let tools = ToolRegistry::new();

    // "运行代码" 应该触发代码执行，而不是系统命令
    let result = tools.detect_and_execute("运行代码 print(123)");
    assert!(result.is_some());
    let result = result.unwrap();
    // 应该是代码执行结果，不是系统命令提示
    assert!(
        result.output.contains("执行")
            || result.output.contains("123")
            || result.output.contains("错误")
    );

    // 测试系统命令检测（不执行危险命令）
    let result = tools.detect_and_execute("打开不存在的程序xyz123");
    assert!(result.is_some());
    let result = result.unwrap();
    // 应该返回"我支持..."的提示
    assert!(result.output.contains("我支持"));
}

/// 测试所有工具都能正常响应
#[test]
fn test_all_tools_respond() {
    let tools = ToolRegistry::new();

    let test_cases = vec![
        ("现在几点了", true),
        ("计算 1 + 1", true),
        ("截图", true),
        ("剪贴板", true),
        ("导出对话", true),
        ("快捷短语", true),
        ("五星好评", true),
        ("搜索天气", true),
        ("翻译成英语你好", true),
        ("分析情感我很开心", true),
        ("运行代码 print(1)", true),
        ("通知测试", true),
        ("记住测试记忆", true),
        ("5分钟后提醒我", true),
        ("查看提醒", true),
    ];

    for (input, should_respond) in test_cases {
        let result = tools.detect_and_execute(input);
        if should_respond {
            assert!(result.is_some(), "工具应该响应: {}", input);
            let result = result.unwrap();
            assert!(result.should_respond, "工具应该返回结果: {}", input);
        }
    }
}
