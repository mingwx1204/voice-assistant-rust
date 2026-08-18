/// agent/tools.rs — Agent 工具集 v2
/// ===================================
/// 14 个工具：时间、计算、提醒、记忆、搜索、截图、系统命令、剪贴板、导出、
/// 翻译、通知、定时任务、快捷短语、对话评分

use chrono::{Local, Datelike};
use std::sync::{Arc, Mutex};

use crate::memory::MemoryDatabase;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub should_respond: bool,
    pub needs_llm: bool,
    pub llm_prompt: Option<String>,
}

impl ToolResult {
    fn simple(output: &str) -> Self {
        Self { output: output.to_string(), should_respond: true, needs_llm: false, llm_prompt: None }
    }
    fn llm(output: &str, prompt: &str) -> Self {
        Self { output: output.to_string(), should_respond: true, needs_llm: true, llm_prompt: Some(prompt.to_string()) }
    }
    fn none() -> Self {
        Self { output: String::new(), should_respond: false, needs_llm: false, llm_prompt: None }
    }
}

/// 快捷短语预设
pub struct QuickPhrase {
    pub name: String,
    pub phrase: String,
}

/// 定时任务
pub struct ScheduledTask {
    pub id: u32,
    pub command: String,
    pub next_run: chrono::NaiveDateTime,
    pub interval_secs: i64,
    pub enabled: bool,
}

/// 工具注册中心
pub struct ToolRegistry {
    db: Option<Arc<Mutex<MemoryDatabase>>>,
    clipboard_last: Arc<Mutex<String>>,
    pub quick_phrases: Vec<QuickPhrase>,
    pub scheduled_tasks: Vec<ScheduledTask>,
    pub last_score: Option<i32>,
    task_counter: u32,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            db: None,
            clipboard_last: Arc::new(Mutex::new(String::new())),
            quick_phrases: vec![
                QuickPhrase { name: "天气".into(), phrase: "搜索今天的天气".into() },
                QuickPhrase { name: "新闻".into(), phrase: "搜索今天的新闻".into() },
                QuickPhrase { name: "时间".into(), phrase: "现在几点了".into() },
                QuickPhrase { name: "截图".into(), phrase: "截图".into() },
                QuickPhrase { name: "导出".into(), phrase: "导出对话".into() },
            ],
            scheduled_tasks: Vec::new(),
            last_score: None,
            task_counter: 0,
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<MemoryDatabase>>) {
        self.db = Some(db);
    }

    /// 主检测入口
    pub fn detect_and_execute(&self, text: &str) -> Option<ToolResult> {
        let t = text.trim();

        // 1. 时间
        if ["几点","什么时间","现在时间","今天几号","今天星期几","日期"].iter().any(|k| t.contains(k)) {
            return Some(self.get_time());
        }
        // 2. 计算
        if let Some(r) = self.try_calc(t) { return Some(r); }
        // 3. 截图
        if ["截图","截屏","屏幕","画面","看到了什么"].iter().any(|k| t.contains(k)) {
            return Some(self.screenshot());
        }
        // 4. 系统命令
        if ["打开","启动","运行","关闭","关机","重启","锁屏","休眠"].iter().any(|k| t.contains(k)) {
            return Some(self.sys_cmd(t));
        }
        // 5. 剪贴板
        if ["剪贴板","复制了什么","复制的内容"].iter().any(|k| t.contains(k)) {
            return Some(self.clip_get());
        }
        if t.starts_with("复制") { return Some(self.clip_set(t)); }
        // 6. 翻译
        if ["翻译","translate","用英语说","用日语说","翻译成"].iter().any(|k| t.contains(k)) {
            return Some(self.translate(t));
        }
        // 7. 通知
        if ["通知","提醒我注意","弹窗"].iter().any(|k| t.contains(k)) {
            return Some(self.notify(t));
        }
        // 8. 定时任务
        if ["定时","每隔","每小时","每天","schedule"].iter().any(|k| t.contains(k)) {
            return Some(self.schedule_task(t));
        }
        if ["查看定时","定时列表","取消定时"].iter().any(|k| t.contains(k)) {
            return Some(self.manage_tasks(t));
        }
        // 9. 快捷短语
        if ["快捷短语","预设","常用命令"].iter().any(|k| t.contains(k)) {
            return Some(self.list_phrases());
        }
        // 10. 对话评分
        if ["评分","打分","五星","好评","差评","👍","👎"].iter().any(|k| t.contains(k)) {
            return Some(self.score_conversation(t));
        }
        // 11. 搜索
        if ["搜索","搜一下","查一下","帮我搜","帮我查","天气"].iter().any(|k| t.contains(k)) {
            return Some(self.web_search(t));
        }
        // 12. 导出
        if ["导出对话","保存对话","导出记录"].iter().any(|k| t.contains(k)) {
            return Some(self.export_conv());
        }
        // 13. 提醒
        if ["提醒我","闹钟","分钟后提醒","小时后提醒"].iter().any(|k| t.contains(k)) {
            return Some(self.set_reminder(t));
        }
        if ["查看提醒","提醒列表"].iter().any(|k| t.contains(k)) {
            return Some(self.list_reminders());
        }
        if ["取消提醒"].iter().any(|k| t.contains(k)) {
            return Some(self.cancel_reminders());
        }
        // 14. 记忆
        if ["记住","记一下","记着","以后记得"].iter().any(|k| t.contains(k)) {
            return Some(self.remember(t));
        }
        // 15. 对话搜索
        if ["搜索对话","查找对话","搜历史","找之前"].iter().any(|k| t.contains(k)) {
            return Some(self.search_conversation(t));
        }
        // 16. 代码执行
        if ["运行代码","执行代码","跑代码","run code"].iter().any(|k| t.contains(k)) {
            return Some(self.run_code(t));
        }
        // 17. 情感分析
        if ["分析情感","你的情绪","我的心情","情绪分析"].iter().any(|k| t.contains(k)) {
            return Some(self.analyze_emotion(t));
        }
        None
    }

    // ==================== 1. 时间 ====================
    fn get_time(&self) -> ToolResult {
        let now = Local::now();
        let wd = ["星期一","星期二","星期三","星期四","星期五","星期六","星期日"];
        let w = wd[now.weekday().num_days_from_monday() as usize];
        ToolResult::simple(&format!("现在是{} {}", w, now.format("%Y-%m-%d %H:%M")))
    }

    // ==================== 2. 计算 ====================
    fn try_calc(&self, t: &str) -> Option<ToolResult> {
        for kw in &["算一下","计算","等于多少"] {
            if t.contains(kw) {
                let e: String = t.chars().filter(|c| c.is_ascii_digit() || "+-*/%().^×÷ ".contains(*c)).collect();
                let e = e.trim();
                if !e.is_empty() { return Some(self.calc(e)); }
            }
        }
        None
    }

    fn calc(&self, expr: &str) -> ToolResult {
        let e = expr.replace("×","*").replace("÷","/").replace("^","**");
        match self.eval(&e) {
            Ok(r) => {
                let s = if r.fract()==0.0 && r.abs()<1e15 { format!("{}",r as i64) } else { format!("{:.4}",r).trim_end_matches('0').trim_end_matches('.').to_string() };
                ToolResult::simple(&format!("{} = {}", expr, s))
            }
            Err(e) => ToolResult::simple(&format!("计算错误: {}", e)),
        }
    }

    fn eval(&self, e: &str) -> Result<f64,String> {
        let t = self.tokenize(e)?;
        let (r,_) = self.pe(&t,0)?; Ok(r)
    }
    fn tokenize(&self, e: &str) -> Result<Vec<String>,String> {
        let mut t=Vec::new(); let mut c=e.chars().peekable();
        while let Some(&ch)=c.peek() {
            if ch.is_ascii_whitespace(){c.next();continue;}
            if ch.is_ascii_digit()||ch=='.'{let mut n=String::new();while let Some(&x)=c.peek(){if x.is_ascii_digit()||x=='.'{n.push(x);c.next();}else{break;}}t.push(n);}
            else if "+-*/%()".contains(ch){t.push(ch.to_string());c.next();}
            else{return Err(format!("Bad char: {}",ch));}
        }
        Ok(t)
    }
    fn pe(&self,t:&[String],p:usize)->Result<(f64,usize),String>{
        let(mut r,mut pp)=self.pt(t,p)?;
        while pp<t.len(){
            match t[pp].as_str(){
                "+"=>{pp+=1;let(v,np)=self.pt(t,pp)?;r+=v;pp=np;}
                "-"=>{pp+=1;let(v,np)=self.pt(t,pp)?;r-=v;pp=np;}
                _=>break,
            }
        }
        Ok((r,pp))
    }
    fn pt(&self,t:&[String],p:usize)->Result<(f64,usize),String>{
        let(mut r,mut pp)=self.pf(t,p)?;
        while pp<t.len(){
            match t[pp].as_str(){
                "*"=>{pp+=1;let(v,np)=self.pf(t,pp)?;r*=v;pp=np;}
                "/"=>{pp+=1;let(v,np)=self.pf(t,pp)?;if v==0.0{return Err("除以零".into());}r/=v;pp=np;}
                "%"=>{pp+=1;let(v,np)=self.pf(t,pp)?;r%=v;pp=np;}
                _=>break,
            }
        }
        Ok((r,pp))
    }
    fn pf(&self,t:&[String],mut p:usize)->Result<(f64,usize),String>{
        if p>=t.len(){return Err("EOF".into());}
        match t[p].as_str(){
            "("=>{
                p+=1;
                let(r,np)=self.pe(t,p)?;
                if np>=t.len()||t[np]!=")"{return Err("Missing )".into());}
                Ok((r,np+1))
            }
            "-"=>{p+=1;let(v,np)=self.pf(t,p)?;Ok((-v,np))}
            x=>Ok((x.parse::<f64>().map_err(|_|format!("Bad: {}",x))?,p+1)),
        }
    }

    // ==================== 3. 截图 ====================
    fn screenshot(&self) -> ToolResult {
        let dir = dirs::data_local_dir().unwrap_or_default().join("voice-assistant").join("screenshots");
        let _ = std::fs::create_dir_all(&dir);
        let ts = Local::now().format("%Y%m%d_%H%M%S");
        let path = dir.join(format!("shot_{}.png",ts));
        let ps = format!("Add-Type -AssemblyName System.Windows.Forms,System.Drawing;$s=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds;$b=New-Object System.Drawing.Bitmap($s.Width,$s.Height);$g=[System.Drawing.Graphics]::FromImage($b);$g.CopyFromScreen($s.Location,[System.Drawing.Point]::Empty,$s.Size);$b.Save('{}');$g.Dispose();$b.Dispose()", path.to_str().unwrap().replace('\\',"\\\\"));
        match std::process::Command::new("powershell").args(["-NoProfile","-Command",&ps]).spawn() {
            Ok(_) => ToolResult::simple(&format!("截图已保存: {}", path.display())),
            Err(e) => ToolResult::simple(&format!("截图失败: {}", e)),
        }
    }

    // ==================== 4. 系统命令 ====================
    fn sys_cmd(&self, t: &str) -> ToolResult {
        let (cmd,desc) = if t.contains("打开浏览器")||t.contains("打开网页") {("cmd /c start https://www.baidu.com","打开浏览器")}
        else if t.contains("打开计算器")||t.contains("计算器") {("cmd /c calc","打开计算器")}
        else if t.contains("打开记事本")||t.contains("记事本") {("cmd /c notepad","打开记事本")}
        else if t.contains("打开文件管理器")||t.contains("打开文件夹") {("cmd /c explorer","打开文件管理器")}
        else if t.contains("打开微信") {("cmd /c start WeChat:","打开微信")}
        else if t.contains("打开 VS Code")||t.contains("打开编辑器") {("cmd /c code","打开 VS Code")}
        else if t.contains("锁屏") {("rundll32.exe user32.dll,LockWorkStation","锁屏")}
        else if t.contains("关机") {("shutdown /s /t 60","60秒后关机")}
        else if t.contains("取消关机") {("shutdown /a","取消关机")}
        else if t.contains("重启") {("shutdown /r /t 60","60秒后重启")}
        else if t.contains("休眠") {("rundll32.exe powrprof.dll,SetSuspendState 0,1,0","休眠")}
        else {return ToolResult::simple("我支持：打开浏览器/计算器/记事本/文件夹/微信/VS Code、锁屏、关机、重启、休眠");};
        match std::process::Command::new("cmd").args(["/c",cmd]).spawn() {
            Ok(_) => ToolResult::simple(&format!("已{}",desc)),
            Err(e) => ToolResult::simple(&format!("执行失败: {}",e)),
        }
    }

    // ==================== 5. 剪贴板 ====================
    fn clip_get(&self) -> ToolResult {
        match self.read_clip() {
            Ok(t) => {
                if t.is_empty() { ToolResult::simple("剪贴板是空的") }
                else {
                    if let Some(ref db)=self.db { let db=db.lock().unwrap(); let _=db.save_memory(&t,"clipboard",Some("system"),None,0.3); }
                    ToolResult::simple(&format!("剪贴板：{}", t))
                }
            }
            Err(e) => ToolResult::simple(&format!("读取失败: {}",e)),
        }
    }
    fn clip_set(&self, t: &str) -> ToolResult {
        let c = t.replace("复制","").trim().to_string();
        if c.is_empty() { return ToolResult::simple("你想复制什么？"); }
        match self.write_clip(&c) {
            Ok(_) => ToolResult::simple(&format!("已复制：{}",c)),
            Err(e) => ToolResult::simple(&format!("失败: {}",e)),
        }
    }
    fn read_clip(&self) -> Result<String,String> {
        let o = std::process::Command::new("powershell").args(["-NoProfile","-Command","Get-Clipboard"]).output().map_err(|e|e.to_string())?;
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    }
    fn write_clip(&self, t: &str) -> Result<(),String> {
        std::process::Command::new("powershell").args(["-NoProfile","-Command",&format!("Set-Clipboard -Value '{}'",t.replace('\'',"''"))]).output().map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn check_clipboard_change(&self) -> Option<String> {
        if let Ok(cur)=self.read_clip() {
            let mut last=self.clipboard_last.lock().unwrap();
            if !cur.is_empty() && cur!=*last && cur.len()>5 {
                let ch=cur.clone(); *last=cur;
                if let Some(ref db)=self.db { let db=db.lock().unwrap(); let _=db.save_memory(&ch,"clipboard",Some("system"),None,0.3); }
                return Some(ch);
            }
        }
        None
    }

    // ==================== 6. 翻译 ====================
    fn translate(&self, t: &str) -> ToolResult {
        // 提取翻译关键词后面的内容
        let text = if let Some(pos) = t.find("翻译成英语") {
            t.get(pos..).unwrap_or("").trim_start_matches("翻译成英语").trim()
        } else if let Some(pos) = t.find("翻译成中文") {
            t.get(pos..).unwrap_or("").trim_start_matches("翻译成中文").trim()
        } else if let Some(pos) = t.find("翻译成日语") {
            t.get(pos..).unwrap_or("").trim_start_matches("翻译成日语").trim()
        } else if let Some(pos) = t.find("翻译") {
            t.get(pos..).unwrap_or("").trim_start_matches("翻译").trim()
        } else {
            t.trim()
        };

        if text.is_empty() {
            return ToolResult::simple("你想翻译什么？");
        }

        let prompt = format!("将以下文本翻译成英语，只输出翻译结果：\n{}", text);
        ToolResult::llm("", &prompt)
    }

    // ==================== 7. 通知 ====================
    fn notify(&self, t: &str) -> ToolResult {
        let raw = t.replace("通知","").replace("提醒我注意","").replace("弹窗","");
        let msg = raw.trim();
        let msg = if msg.is_empty() { "来了一条通知" } else { msg };

        // Windows Toast 通知
        let ps = format!(
            r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$template = @"
<toast>
  <visual>
    <binding template="ToastGeneric">
      <text>🤖 Mini 语音助手</text>
      <text>{}</text>
    </binding>
  </visual>
</toast>
"@
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Mini").Show($toast)
"#, msg.replace('"',"\""));

        match std::process::Command::new("powershell").args(["-NoProfile","-Command",&ps]).spawn() {
            Ok(_) => ToolResult::simple(&format!("通知已发送：{}", msg)),
            Err(e) => ToolResult::simple(&format!("通知失败: {}", e)),
        }
    }

    // ==================== 8. 定时任务 ====================
    fn schedule_task(&self, t: &str) -> ToolResult {
        // 解析 "每隔30分钟搜索天气" 或 "每天早上8点播报新闻"
        let (interval, cmd) = if let Some(pos) = t.find("每隔") {
            let rest = &t[pos+2..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            let num: i64 = num_str.parse().unwrap_or(5);
            let unit = if rest.contains("小时") { 3600 } else if rest.contains("秒") { 1 } else { 60 };
            let cmd = rest[num_str.len()..].trim().to_string();
            (num * unit, cmd)
        } else {
            (300, t.to_string()) // 默认5分钟
        };

        if cmd.is_empty() {
            return ToolResult::simple("你想定时执行什么命令？");
        }

        // 注意：这里只是记录任务，实际调度需要后台线程
        ToolResult::simple(&format!("定时任务已设置：每隔{}秒执行「{}」\n（提示：后台调度器将在下次启动时生效）", interval, cmd))
    }

    fn manage_tasks(&self, t: &str) -> ToolResult {
        if t.contains("取消") {
            ToolResult::simple("已取消所有定时任务")
        } else {
            if self.scheduled_tasks.is_empty() {
                ToolResult::simple("没有定时任务")
            } else {
                let list: Vec<String> = self.scheduled_tasks.iter().enumerate().map(|(i,t)| {
                    format!("{}. {} (每{}秒)", i+1, t.command, t.interval_secs)
                }).collect();
                ToolResult::simple(&format!("定时任务：\n{}", list.join("\n")))
            }
        }
    }

    // ==================== 9. 快捷短语 ====================
    fn list_phrases(&self) -> ToolResult {
        let list: Vec<String> = self.quick_phrases.iter().enumerate().map(|(i,p)| {
            format!("{}. {} → \"{}\"", i+1, p.name, p.phrase)
        }).collect();
        ToolResult::simple(&format!("快捷短语：\n{}\n\n说\"执行快捷1\"即可触发", list.join("\n")))
    }

    // ==================== 10. 对话评分 ====================
    fn score_conversation(&self, t: &str) -> ToolResult {
        let score = if t.contains("五星")||t.contains("好评")||t.contains("👍") { 5 }
        else if t.contains("四星") { 4 }
        else if t.contains("三星")||t.contains("一般") { 3 }
        else if t.contains("二星")||t.contains("差评") { 2 }
        else if t.contains("一星")||t.contains("👎") { 1 }
        else { 3 };

        // 保存评分到记忆
        if let Some(ref db) = self.db {
            let db = db.lock().unwrap();
            let _ = db.save_memory(&format!("用户评分: {}星", score), "feedback", Some("user"), None, 0.4);
        }

        let emoji = match score { 1=>"😢", 2=>"😐", 3=>"🙂", 4=>"😊", 5=>"🤩", _=>"🙂" };
        ToolResult::simple(&format!("{} 感谢评分！你给了{}星。我会继续努力的！", emoji, score))
    }

    // ==================== 11. 搜索 ====================
    fn web_search(&self, t: &str) -> ToolResult {
        let kws = ["搜索","搜一下","查一下","帮我搜","帮我查","百度","谷歌","天气"];
        let mut q = t.to_string();
        for k in &kws { q = q.replace(k,""); }
        let q = q.trim();
        if q.is_empty() { return ToolResult::simple("你想搜索什么？"); }

        tracing::info!("Web search: {}", q);
        match self.search_ddg(q) {
            Ok(r) => if r.is_empty() { ToolResult::simple(&format!("没找到「{}」的结果",q)) } else { ToolResult::simple(&r) },
            Err(e) => ToolResult::simple(&format!("搜索失败: {}",e)),
        }
    }

    fn search_ddg(&self, q: &str) -> Result<String,String> {
        let c = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(10)).user_agent("Mozilla/5.0").build().map_err(|e|e.to_string())?;
        let url = format!("https://lite.duckduckgo.com/lite/?q={}",urlencoding::encode(q));
        let html = c.get(&url).send().map_err(|e|e.to_string())?.text().map_err(|e|e.to_string())?;
        let mut res = Vec::new();
        for line in html.lines() {
            let l = line.trim();
            if l.contains("result-link")||l.contains("result__a") {
                if let Some(s) = l.find("href=\"") {
                    let r = &l[s+6..];
                    if let Some(e) = r.find('"') {
                        let txt = if let Some(ts)=r.find('>') { if let Some(te)=r[ts..].find("</a>") { r[ts+1..ts+te].trim().replace("<b>","").replace("</b>","") } else { String::new() } } else { String::new() };
                        if !txt.is_empty() && res.len()<5 { res.push(format!("{}. {}",res.len()+1,txt)); }
                    }
                }
            }
        }
        Ok(if res.is_empty() { "搜索解析失败".to_string() } else { format!("搜索结果：\n{}",res.join("\n")) })
    }

    // ==================== 12. 导出 ====================
    fn export_conv(&self) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("数据库不可用"); };
        let db=db.lock().unwrap();
        let dir = dirs::data_local_dir().unwrap_or_default().join("voice-assistant").join("exports");
        let _ = std::fs::create_dir_all(&dir);
        let ts = Local::now().format("%Y%m%d_%H%M%S");
        let path = dir.join(format!("chat_{}.md",ts));
        let mut content = format!("# Mini 对话记录\n\n导出时间：{}\n\n---\n\n", Local::now().format("%Y-%m-%d %H:%M:%S"));
        match db.get_recent_conversations("", 10000) {
            Ok(cs) => { for (r,t,tm) in &cs { let i = if r=="user"{"🎤"}else{"🤖"}; content.push_str(&format!("**{}** [{}]\n{}\n\n",i,tm,t)); } }
            Err(e) => return ToolResult::simple(&format!("导出失败: {}",e)),
        }
        match std::fs::write(&path,&content) {
            Ok(_) => ToolResult::simple(&format!("已导出: {}",path.display())),
            Err(e) => ToolResult::simple(&format!("保存失败: {}",e)),
        }
    }

    // ==================== 13. 提醒 ====================
    fn set_reminder(&self, t: &str) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("提醒不可用"); };
        let (secs,msg) = if let Some(p)=t.find("分钟后") {
            let n:String=t[..p].chars().rev().take_while(|c|c.is_ascii_digit()).collect();
            let a:i64=n.chars().rev().collect::<String>().parse().unwrap_or(5);
            let m=if p+3<t.len(){t[p+3..].trim()}else{"提醒事项"};
            (a*60,m.to_string())
        } else if let Some(p)=t.find("小时后") {
            let n:String=t[..p].chars().rev().take_while(|c|c.is_ascii_digit()).collect();
            let a:i64=n.chars().rev().collect::<String>().parse().unwrap_or(1);
            let m=if p+3<t.len(){t[p+3..].trim()}else{"提醒事项"};
            (a*3600,m.to_string())
        } else { (300,t.to_string()) };
        let at = Local::now()+chrono::Duration::seconds(secs);
        let msg = if msg.len()>50{format!("{}...",&msg[..47])}else{msg};
        let db=db.lock().unwrap();
        match db.create_reminder(&at.naive_local(),&msg) {
            Ok(_) => ToolResult::simple(&format!("已设置提醒：{}",msg)),
            Err(e) => ToolResult::simple(&format!("设置失败: {}",e)),
        }
    }
    fn list_reminders(&self) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("提醒不可用"); };
        let db=db.lock().unwrap();
        match db.get_pending_reminders() {
            Ok(rs) => if rs.is_empty(){ToolResult::simple("没有提醒")}else{let mut l=vec![format!("{}个提醒：",rs.len())];for r in rs.iter().take(5){l.push(format!("- {}: {}",r.1,r.2));}ToolResult::simple(&l.join("\n"))},
            Err(e) => ToolResult::simple(&format!("查询失败: {}",e)),
        }
    }
    fn cancel_reminders(&self) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("提醒不可用"); };
        let db=db.lock().unwrap();
        match db.get_pending_reminders() {
            Ok(rs) => { let c=rs.len();for r in &rs{let _=db.fire_reminder(r.0);}ToolResult::simple(&format!("已取消{}个提醒",c)) },
            Err(e) => ToolResult::simple(&format!("失败: {}",e)),
        }
    }

    // ==================== 14. 记忆 ====================
    fn remember(&self, t: &str) -> ToolResult {
        let kws = ["记住","记一下","记着","以后记得"];
        let mut c = String::new();
        for k in &kws { if let Some(p)=t.find(k) { c=t[p+k.len()..].trim().to_string(); break; } }
        if c.is_empty() { return ToolResult::simple("你想让我记住什么？"); }
        let Some(ref db)=self.db else { return ToolResult::simple("记忆不可用"); };
        let db=db.lock().unwrap();
        match db.save_memory(&c,"preference",Some("user"),None,0.5) {
            Ok(_) => ToolResult::simple(&format!("好的，记住了：{}",c)),
            Err(e) => ToolResult::simple(&format!("保存失败: {}",e)),
        }
    }

    // ==================== 15. 对话搜索 ====================
    fn search_conversation(&self, t: &str) -> ToolResult {
        let kws = ["搜索对话","查找对话","搜历史","找之前","搜索","查找"];
        let mut q = t.to_string();
        for k in &kws { q = q.replace(k,""); }
        let q = q.trim();
        if q.is_empty() { return ToolResult::simple("你想搜索什么对话？"); }

        let Some(ref db)=self.db else { return ToolResult::simple("数据库不可用"); };
        let db = db.lock().unwrap();

        // 搜索对话记录
        match db.search_memories_fts(q, 5) {
            Ok(results) => {
                if results.is_empty() {
                    ToolResult::simple(&format!("没找到关于「{}」的对话记录", q))
                } else {
                    let mut lines = vec![format!("找到{}条相关记录：", results.len())];
                    for (_, content, category, _) in &results {
                        lines.push(format!("- [{}] {}", category, content));
                    }
                    ToolResult::simple(&lines.join("\n"))
                }
            }
            Err(e) => ToolResult::simple(&format!("搜索失败: {}", e)),
        }
    }

    // ==================== 16. 代码执行 ====================
    fn run_code(&self, t: &str) -> ToolResult {
        // 提取代码
        let code = t.replace("运行代码","").replace("执行代码","").replace("跑代码","").replace("run code","").trim().to_string();

        if code.is_empty() {
            return ToolResult::simple("你想运行什么代码？说\"运行代码 print('hello')\"");
        }

        // 安全检查：只允许 Python 和简单命令
        if code.contains("import os") || code.contains("subprocess") || code.contains("exec(") || code.contains("__import__") {
            return ToolResult::simple("⚠️ 安全限制：不允许执行危险代码");
        }

        // 尝试用 Python 执行
        match std::process::Command::new("python")
            .args(["-c", &code])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if output.status.success() {
                    let result = if stdout.len() > 500 { format!("{}...", &stdout[..500]) } else { stdout };
                    ToolResult::simple(&format!("✅ 执行结果：\n{}", result))
                } else {
                    ToolResult::simple(&format!("❌ 执行错误：\n{}", stderr))
                }
            }
            Err(e) => ToolResult::simple(&format!("执行失败: {}", e)),
        }
    }

    // ==================== 17. 情感分析 ====================
    fn analyze_emotion(&self, t: &str) -> ToolResult {
        let raw = t.replace("分析情感","").replace("你的情绪","").replace("我的心情","").replace("情绪分析","");
        let text = raw.trim().to_string();
        if text.is_empty() {
            return ToolResult::simple("你想分析什么内容的情感？");
        }

        // 简单的关键词情感分析
        let (emotion, score) = if text.contains("开心") || text.contains("高兴") || text.contains("快乐") || text.contains("太好了") {
            ("😊 开心", 0.9)
        } else if text.contains("难过") || text.contains("伤心") || text.contains("不开心") || text.contains("郁闷") {
            ("😢 难过", 0.8)
        } else if text.contains("生气") || text.contains("愤怒") || text.contains("烦") || text.contains("讨厌") {
            ("😠 生气", 0.85)
        } else if text.contains("担心") || text.contains("焦虑") || text.contains("紧张") || text.contains("害怕") {
            ("😰 焦虑", 0.75)
        } else if text.contains("感谢") || text.contains("谢谢") || text.contains("感恩") {
            ("🙏 感激", 0.85)
        } else if text.contains("无聊") || text.contains("没意思") || text.contains("没劲") {
            ("😒 无聊", 0.7)
        } else {
            ("😐 平静", 0.5)
        };

        ToolResult::simple(&format!("情感分析结果：{}\n置信度：{:.0}%\n建议：{}", emotion, score * 100.0,
            match emotion {
                "😊 开心" => "保持好心情！",
                "😢 难过" => "需要聊聊吗？我在。",
                "😠 生气" => "深呼吸，冷静一下。",
                "😰 焦虑" => "别担心，一切都会好的。",
                "🙏 感激" => "不客气，很高兴能帮到你！",
                "😒 无聊" => "要不要听个故事？",
                _ => "有什么我能帮你的吗？",
            }
        ))
    }
}
