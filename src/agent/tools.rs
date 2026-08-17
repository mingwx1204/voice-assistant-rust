/// agent/tools.rs — Agent 工具集
/// ================================
/// 完整工具集：时间、计算、提醒、记忆、搜索、截图、系统命令、剪贴板、导出

use chrono::{Local, Datelike};
use std::sync::{Arc, Mutex};

use crate::memory::MemoryDatabase;

/// 工具执行结果
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub should_respond: bool,
}

/// 工具注册中心
pub struct ToolRegistry {
    db: Option<Arc<Mutex<MemoryDatabase>>>,
    clipboard_last: Arc<Mutex<String>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            db: None,
            clipboard_last: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<MemoryDatabase>>) {
        self.db = Some(db);
    }

    /// 检测并执行工具
    pub fn detect_and_execute(&self, user_text: &str) -> Option<ToolResult> {
        let text = user_text.trim();

        // ===== 时间 =====
        if ["几点", "什么时间", "现在时间", "今天几号", "今天星期几", "日期"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.get_time());
        }

        // ===== 数学计算 =====
        if let Some(result) = self.try_calculate(text) {
            return Some(result);
        }

        // ===== 截图问答 =====
        if ["截图", "截屏", "屏幕", "画面", "看到了什么"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.screenshot());
        }

        // ===== 系统命令 =====
        if ["打开", "启动", "运行", "关闭", "关机", "重启", "锁屏", "休眠"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.system_command(text));
        }

        // ===== 剪贴板 =====
        if ["剪贴板", "复制了什么", "复制的内容", "粘贴板"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.clipboard_get());
        }
        if text.starts_with("复制") || text.starts_with("记住这段") {
            return Some(self.clipboard_set(text));
        }

        // ===== 联网搜索 =====
        if ["搜索", "搜一下", "查一下", "查找", "帮我搜", "帮我查", "网上搜", "百度", "谷歌", "天气"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.web_search(text));
        }

        // ===== 对话导出 =====
        if ["导出对话", "保存对话", "导出记录", "保存记录"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.export_conversation());
        }

        // ===== 提醒 =====
        if ["提醒我", "闹钟", "分钟后提醒", "小时后提醒"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.set_reminder(text));
        }
        if ["查看提醒", "有什么提醒", "提醒列表"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.list_reminders());
        }
        if ["取消提醒", "删除提醒"].iter().any(|kw| text.contains(kw)) {
            return Some(self.cancel_reminders());
        }

        // ===== 记忆 =====
        if ["记住", "记一下", "记着", "以后记得"]
            .iter().any(|kw| text.contains(kw))
        {
            return Some(self.remember(text));
        }

        None
    }

    // ===================================================================
    // ⏰ 时间
    // ===================================================================
    fn get_time(&self) -> ToolResult {
        let now = Local::now();
        let weekdays = ["星期一","星期二","星期三","星期四","星期五","星期六","星期日"];
        let weekday = weekdays[now.weekday().num_days_from_monday() as usize];
        ToolResult {
            output: format!("现在是{} {}", weekday, now.format("%Y-%m-%d %H:%M")),
            should_respond: true,
        }
    }

    // ===================================================================
    // 🧮 数学计算
    // ===================================================================
    fn try_calculate(&self, text: &str) -> Option<ToolResult> {
        for keyword in &["算一下", "计算", "等于多少"] {
            if text.contains(keyword) {
                let expr: String = text.chars()
                    .filter(|c| c.is_ascii_digit() || "+-*/%().^×÷ ".contains(*c))
                    .collect();
                let expr = expr.trim();
                if !expr.is_empty() {
                    return Some(self.calculate(expr));
                }
            }
        }
        None
    }

    fn calculate(&self, expression: &str) -> ToolResult {
        let expr = expression.replace("×", "*").replace("÷", "/").replace("^", "**");
        match self.eval_expression(&expr) {
            Ok(result) => {
                let s = if result.fract() == 0.0 && result.abs() < 1e15 {
                    format!("{}", result as i64)
                } else {
                    format!("{:.4}", result).trim_end_matches('0').trim_end_matches('.').to_string()
                };
                ToolResult { output: format!("{} = {}", expression, s), should_respond: true }
            }
            Err(e) => ToolResult { output: format!("计算错误: {}", e), should_respond: true },
        }
    }

    fn eval_expression(&self, expr: &str) -> Result<f64, String> {
        let tokens = self.tokenize(expr)?;
        let (result, _) = self.parse_expr(&tokens, 0)?;
        Ok(result)
    }

    fn tokenize(&self, expr: &str) -> Result<Vec<String>, String> {
        let mut tokens = Vec::new();
        let mut chars = expr.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_whitespace() { chars.next(); continue; }
            if c.is_ascii_digit() || c == '.' {
                let mut num = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' { num.push(ch); chars.next(); } else { break; }
                }
                tokens.push(num);
            } else if "+-*/%()".contains(c) {
                tokens.push(c.to_string());
                chars.next();
            } else {
                return Err(format!("Unexpected: {}", c));
            }
        }
        Ok(tokens)
    }

    fn parse_expr(&self, tokens: &[String], pos: usize) -> Result<(f64, usize), String> {
        let (mut result, mut p) = self.parse_term(tokens, pos)?;
        while p < tokens.len() {
            match tokens[p].as_str() {
                "+" => { p += 1; let (v, np) = self.parse_term(tokens, p)?; result += v; p = np; }
                "-" => { p += 1; let (v, np) = self.parse_term(tokens, p)?; result -= v; p = np; }
                _ => break,
            }
        }
        Ok((result, p))
    }

    fn parse_term(&self, tokens: &[String], pos: usize) -> Result<(f64, usize), String> {
        let (mut result, mut p) = self.parse_factor(tokens, pos)?;
        while p < tokens.len() {
            match tokens[p].as_str() {
                "*" => { p += 1; let (v, np) = self.parse_factor(tokens, p)?; result *= v; p = np; }
                "/" => { p += 1; let (v, np) = self.parse_factor(tokens, p)?; if v == 0.0 { return Err("除以零".into()); } result /= v; p = np; }
                "%" => { p += 1; let (v, np) = self.parse_factor(tokens, p)?; if v == 0.0 { return Err("除以零".into()); } result %= v; p = np; }
                _ => break,
            }
        }
        Ok((result, p))
    }

    fn parse_factor(&self, tokens: &[String], mut pos: usize) -> Result<(f64, usize), String> {
        if pos >= tokens.len() { return Err("Unexpected end".into()); }
        match tokens[pos].as_str() {
            "(" => { pos += 1; let (r, p) = self.parse_expr(tokens, pos)?; if p >= tokens.len() || tokens[p] != ")" { return Err("Missing )".into()); } Ok((r, p + 1)) }
            "-" => { pos += 1; let (v, p) = self.parse_factor(tokens, pos)?; Ok((-v, p)) }
            t => Ok((t.parse::<f64>().map_err(|_| format!("Bad token: {}", t))?, pos + 1)),
        }
    }

    // ===================================================================
    // 📸 截图
    // ===================================================================
    fn screenshot(&self) -> ToolResult {
        // 使用 Windows 原生 API 截图
        match self.take_screenshot() {
            Ok(path) => ToolResult {
                output: format!("截图已保存到: {}. 请查看截图内容并描述。", path),
                should_respond: true,
            },
            Err(e) => ToolResult {
                output: format!("截图失败: {}", e),
                should_respond: true,
            },
        }
    }

    fn take_screenshot(&self) -> Result<String, String> {
        let output_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("voice-assistant").join("screenshots");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let path = output_dir.join(format!("screenshot_{}.png", timestamp));
        let path_str = path.to_str().ok_or("Invalid path")?.to_string();

        // 使用 PowerShell 截图（Windows 原生方式）
        let ps_script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
$bitmap.Save('{}')
$graphics.Dispose()
$bitmap.Dispose()
"#,
            path_str.replace('\\', "\\\\")
        );

        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| e.to_string())?;

        if path.exists() {
            Ok(path_str)
        } else {
            Err("截图文件未生成".to_string())
        }
    }

    // ===================================================================
    // 💻 系统命令
    // ===================================================================
    fn system_command(&self, text: &str) -> ToolResult {
        // 安全的命令白名单
        let (cmd, desc) = if text.contains("打开浏览器") || text.contains("打开网页") {
            ("cmd /c start https://www.baidu.com", "打开浏览器")
        } else if text.contains("打开计算器") || text.contains("计算器") {
            ("cmd /c calc", "打开计算器")
        } else if text.contains("打开记事本") || text.contains("记事本") {
            ("cmd /c notepad", "打开记事本")
        } else if text.contains("打开文件管理器") || text.contains("打开文件夹") {
            ("cmd /c explorer", "打开文件管理器")
        } else if text.contains("打开微信") {
            ("cmd /c start WeChat:", "打开微信")
        } else if text.contains("打开 VS Code") || text.contains("打开编辑器") {
            ("cmd /c code", "打开 VS Code")
        } else if text.contains("锁屏") {
            ("cmd /rundll32.exe user32.dll,LockWorkStation", "锁屏")
        } else if text.contains("关机") {
            ("shutdown /s /t 60", "关机（60秒后）")
        } else if text.contains("取消关机") {
            ("shutdown /a", "取消关机")
        } else if text.contains("重启") {
            ("shutdown /r /t 60", "重启（60秒后）")
        } else if text.contains("休眠") {
            ("rundll32.exe powrprof.dll,SetSuspendState 0,1,0", "休眠")
        } else {
            return ToolResult {
                output: "我支持：打开浏览器/计算器/记事本/文件夹/微信/VS Code、锁屏、关机、重启、休眠".to_string(),
                should_respond: true,
            };
        };

        match std::process::Command::new("cmd").args(["/c", cmd]).spawn() {
            Ok(_) => ToolResult { output: format!("已{}", desc), should_respond: true },
            Err(e) => ToolResult { output: format!("执行失败: {}", e), should_respond: true },
        }
    }

    // ===================================================================
    // 📋 剪贴板
    // ===================================================================
    fn clipboard_get(&self) -> ToolResult {
        match self.read_clipboard() {
            Ok(text) => {
                if text.is_empty() {
                    ToolResult { output: "剪贴板是空的".to_string(), should_respond: true }
                } else {
                    // 自动保存到记忆
                    if let Some(ref db) = self.db {
                        let db = db.lock().unwrap();
                        let _ = db.save_memory(&text, "clipboard", Some("system"), None, 0.3);
                    }
                    ToolResult { output: format!("剪贴板内容：{}", text), should_respond: true }
                }
            }
            Err(e) => ToolResult { output: format!("读取剪贴板失败: {}", e), should_respond: true },
        }
    }

    fn clipboard_set(&self, text: &str) -> ToolResult {
        let content = text.replace("复制", "").replace("记住这段", "").trim().to_string();
        if content.is_empty() {
            return ToolResult { output: "你想复制什么？".to_string(), should_respond: true };
        }
        match self.write_clipboard(&content) {
            Ok(_) => ToolResult { output: format!("已复制到剪贴板：{}", content), should_respond: true },
            Err(e) => ToolResult { output: format!("复制失败: {}", e), should_respond: true },
        }
    }

    fn read_clipboard(&self) -> Result<String, String> {
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard"])
            .output()
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn write_clipboard(&self, text: &str) -> Result<(), String> {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"))])
            .output()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 检查剪贴板变化（后台调用）
    pub fn check_clipboard_change(&self) -> Option<String> {
        if let Ok(current) = self.read_clipboard() {
            let mut last = self.clipboard_last.lock().unwrap();
            if !current.is_empty() && current != *last && current.len() > 5 {
                let changed = current.clone();
                *last = current;
                // 自动存入记忆
                if let Some(ref db) = self.db {
                    let db = db.lock().unwrap();
                    let _ = db.save_memory(&changed, "clipboard", Some("system"), None, 0.3);
                }
                return Some(changed);
            }
        }
        None
    }

    // ===================================================================
    // 🔍 联网搜索
    // ===================================================================
    fn web_search(&self, text: &str) -> ToolResult {
        let keywords = ["搜索","搜一下","查一下","查找","帮我搜","帮我查","网上搜","百度","谷歌","天气"];
        let mut query = text.to_string();
        for kw in &keywords { query = query.replace(kw, ""); }
        let query = query.trim();
        if query.is_empty() {
            return ToolResult { output: "你想搜索什么？".to_string(), should_respond: true };
        }

        tracing::info!("Web search: {}", query);

        match self.search_duckduckgo(query) {
            Ok(results) => {
                if results.is_empty() {
                    ToolResult { output: format!("没找到关于「{}」的结果", query), should_respond: true }
                } else {
                    ToolResult { output: results, should_respond: true }
                }
            }
            Err(e) => {
                tracing::warn!("Search failed: {}", e);
                ToolResult { output: format!("搜索失败: {}", e), should_respond: true }
            }
        }
    }

    fn search_duckduckgo(&self, query: &str) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build().map_err(|e| e.to_string())?;

        let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query));
        let resp = client.get(&url).send().map_err(|e| e.to_string())?;
        let html = resp.text().map_err(|e| e.to_string())?;

        let results = self.parse_ddg_html(&html);
        Ok(results)
    }

    fn parse_ddg_html(&self, html: &str) -> String {
        let mut results = Vec::new();
        for line in html.lines() {
            let line = line.trim();
            if line.contains("result-link") || line.contains("result__a") {
                if let Some(start) = line.find("href=\"") {
                    let rest = &line[start + 6..];
                    if let Some(end) = rest.find("\"") {
                        let href = &rest[..end];
                        let text = if let Some(t_start) = rest.find('>') {
                            if let Some(t_end) = rest[t_start..].find("</a>") {
                                rest[t_start + 1..t_start + t_end].trim()
                                    .replace("<b>", "").replace("</b>", "")
                                    .replace("<strong>", "").replace("</strong>", "")
                            } else { String::new() }
                        } else { String::new() };
                        if !text.is_empty() && !href.is_empty() && results.len() < 5 {
                            results.push(format!("{}. {}", results.len() + 1, text));
                        }
                    }
                }
            }
        }
        if results.is_empty() {
            for line in html.lines() {
                let line = line.trim();
                if line.contains("<a ") && line.contains("href=") && line.contains("http") {
                    if let Some(start) = line.find('>') {
                        let text = line[start + 1..].replace("</a>", "").replace("<b>", "").replace("</b>", "");
                        let text = text.trim().to_string();
                        if text.len() > 10 && results.len() < 5 {
                            results.push(format!("{}. {}", results.len() + 1, text));
                        }
                    }
                }
            }
        }
        if results.is_empty() { "搜索结果解析失败".to_string() }
        else { format!("搜索结果：\n{}", results.join("\n")) }
    }

    // ===================================================================
    // 💾 对话导出
    // ===================================================================
    fn export_conversation(&self) -> ToolResult {
        let Some(ref db) = self.db else {
            return ToolResult { output: "数据库不可用".to_string(), should_respond: true };
        };

        let db = db.lock().unwrap();
        let export_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("voice-assistant").join("exports");
        if let Err(e) = std::fs::create_dir_all(&export_dir) {
            return ToolResult { output: format!("创建导出目录失败: {}", e), should_respond: true };
        }

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let path = export_dir.join(format!("conversation_{}.md", timestamp));

        let mut content = format!("# Mini 语音助手对话记录\n\n导出时间：{}\n\n---\n\n", Local::now().format("%Y-%m-%d %H:%M:%S"));

        // 导出所有会话
        match db.get_recent_conversations("", 10000) {
            Ok(convos) => {
                for (role, text, time) in &convos {
                    let icon = if role == "user" { "🎤" } else { "🤖" };
                    let name = if role == "user" { "用户" } else { "Mini" };
                    content.push_str(&format!("**{}** [{}]\n{}\n\n", name, time, text));
                }
            }
            Err(e) => {
                return ToolResult { output: format!("导出失败: {}", e), should_respond: true };
            }
        }

        match std::fs::write(&path, &content) {
            Ok(_) => ToolResult {
                output: format!("对话已导出到: {}", path.display()),
                should_respond: true,
            },
            Err(e) => ToolResult {
                output: format!("保存失败: {}", e),
                should_respond: true,
            },
        }
    }

    // ===================================================================
    // ⏰ 提醒
    // ===================================================================
    fn set_reminder(&self, text: &str) -> ToolResult {
        let Some(db_arc) = &self.db else {
            return ToolResult { output: "提醒功能不可用".to_string(), should_respond: true };
        };

        let (delta_secs, message) = if let Some(pos) = text.find("分钟后") {
            let before = &text[..pos];
            let num_str: String = before.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
            let amount: i64 = num_str.chars().rev().collect::<String>().parse().unwrap_or(5);
            let msg = if pos + 3 < text.len() { text[pos+3..].trim() } else { "提醒事项" };
            (amount * 60, msg.to_string())
        } else if let Some(pos) = text.find("小时后") {
            let before = &text[..pos];
            let num_str: String = before.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
            let amount: i64 = num_str.chars().rev().collect::<String>().parse().unwrap_or(1);
            let msg = if pos + 3 < text.len() { text[pos+3..].trim() } else { "提醒事项" };
            (amount * 3600, msg.to_string())
        } else {
            (300, text.to_string())
        };

        let remind_at = Local::now() + chrono::Duration::seconds(delta_secs);
        let message = if message.len() > 50 { format!("{}...", &message[..47]) } else { message };

        let db = db_arc.lock().unwrap();
        match db.create_reminder(&remind_at.naive_local(), &message) {
            Ok(_) => ToolResult { output: format!("好的，已设置提醒：{}", message), should_respond: true },
            Err(e) => ToolResult { output: format!("设置提醒失败: {}", e), should_respond: true },
        }
    }

    fn list_reminders(&self) -> ToolResult {
        let Some(db_arc) = &self.db else {
            return ToolResult { output: "提醒功能不可用".to_string(), should_respond: true };
        };
        let db = db_arc.lock().unwrap();
        match db.get_pending_reminders() {
            Ok(reminders) => {
                if reminders.is_empty() {
                    ToolResult { output: "没有待处理的提醒".to_string(), should_respond: true }
                } else {
                    let mut lines = vec![format!("你有{}个待处理的提醒：", reminders.len())];
                    for r in reminders.iter().take(5) {
                        lines.push(format!("- {}: {}", r.1, r.2));
                    }
                    ToolResult { output: lines.join("\n"), should_respond: true }
                }
            }
            Err(e) => ToolResult { output: format!("获取提醒失败: {}", e), should_respond: true },
        }
    }

    fn cancel_reminders(&self) -> ToolResult {
        let Some(db_arc) = &self.db else {
            return ToolResult { output: "提醒功能不可用".to_string(), should_respond: true };
        };
        let db = db_arc.lock().unwrap();
        match db.get_pending_reminders() {
            Ok(reminders) => {
                let count = reminders.len();
                for r in &reminders { let _ = db.fire_reminder(r.0); }
                ToolResult { output: format!("已取消{}个提醒", count), should_respond: true }
            }
            Err(e) => ToolResult { output: format!("取消提醒失败: {}", e), should_respond: true },
        }
    }

    // ===================================================================
    // 📝 记忆
    // ===================================================================
    fn remember(&self, text: &str) -> ToolResult {
        let keywords = ["记住","记一下","记着","以后记得"];
        let mut content = String::new();
        for kw in &keywords {
            if let Some(pos) = text.find(kw) {
                content = text[pos + kw.len()..].trim().to_string();
                break;
            }
        }
        if content.is_empty() {
            return ToolResult { output: "你想让我记住什么？".to_string(), should_respond: true };
        }
        let Some(db_arc) = &self.db else {
            return ToolResult { output: "记忆功能不可用".to_string(), should_respond: true };
        };
        let db = db_arc.lock().unwrap();
        match db.save_memory(&content, "preference", Some("user"), None, 0.5) {
            Ok(_) => ToolResult { output: format!("好的，我记住了：{}", content), should_respond: true },
            Err(e) => ToolResult { output: format!("保存记忆失败: {}", e), should_respond: true },
        }
    }
}
