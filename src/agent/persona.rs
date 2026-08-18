/// agent/persona.rs — Agent 人格设定
/// ====================================
/// Mini 语音助手的性格、能力、语气和行为约束。
use chrono::{Datelike, Local, Timelike, Weekday};

use crate::config::AgentConfig;

/// Agent 人格定义
pub struct AgentPersona {
    pub name: String,
    pub personality: String,
    pub max_sentences: usize,
}

impl AgentPersona {
    /// 从配置创建
    pub fn from_config(config: &AgentConfig) -> Self {
        Self {
            name: config.name.clone(),
            personality: config.personality.clone(),
            max_sentences: config.max_reply_sentences,
        }
    }

    /// 获取系统提示词
    pub fn get_system_prompt(&self, memory_context: &str) -> String {
        let now = Local::now();
        let weekday_str = match now.weekday() {
            Weekday::Mon => "星期一",
            Weekday::Tue => "星期二",
            Weekday::Wed => "星期三",
            Weekday::Thu => "星期四",
            Weekday::Fri => "星期五",
            Weekday::Sat => "星期六",
            Weekday::Sun => "星期日",
        };
        let time_str = format!("{} {}", now.format("%Y年%m月%d日 %H:%M"), weekday_str);

        let memory_section = if !memory_context.is_empty() {
            format!("\n与当前对话相关的记忆：\n{}", memory_context)
        } else {
            String::new()
        };

        format!(
            r#"你是 {name}，一个{personality}。

你的核心能力：
1. 用简洁、自然的中文回答问题，像在打电话一样
2. 记得用户之前告诉你的信息
3. 可以查询时间、做数学计算、设定提醒
4. 可以联网搜索实时信息（天气、新闻、知识等）
5. 不确定的事情诚实说"我不太确定"

回答规则：
- 回复控制在 {max_sentences} 句话以内
- 用口语化的中文，不要用书面语
- 不要使用 markdown 格式
- 不要使用列表或编号格式
- 不要说"作为 AI 助手"之类的话
- 不知道就说"这个我不太清楚"，不要编造
- 如果用户问了你之前记住的信息，自然地引用
- 如果需要实时信息，告诉用户"我帮你搜一下"
{memory_section}

当前时间：{current_time}"#,
            name = self.name,
            personality = self.personality,
            max_sentences = self.max_sentences,
            memory_section = memory_section,
            current_time = time_str,
        )
    }

    /// 获取问候语
    pub fn get_greeting(&self) -> String {
        let hour = Local::now().hour();
        let greeting = match hour {
            6..=11 => "早上好",
            12..=17 => "下午好",
            18..=22 => "晚上好",
            _ => "你好",
        };
        format!(
            "{}！我是{}，你的语音助手。有什么可以帮你的？",
            greeting, self.name
        )
    }

    /// 获取无语音响应
    pub fn get_no_voice_response(&self) -> String {
        "抱歉，我没听清。你能再说一遍吗？".to_string()
    }

    /// 获取服务不可用响应
    pub fn get_service_unavailable_response(&self) -> String {
        "抱歉，模型服务似乎不可用。我现在无法回答问题。".to_string()
    }

    /// 获取错误响应
    pub fn get_error_response(&self) -> String {
        "抱歉，我遇到了一些问题。请稍后再试。".to_string()
    }
}
