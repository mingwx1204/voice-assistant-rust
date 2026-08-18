/// agent/tools.rs — Agent 工具集 v3 (优化版)
/// ============================================
/// 性能优化：关键词 HashMap 查找、搜索缓存、预分配字符串

use chrono::{Datelike, Local};
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
}

pub struct ToolRegistry {
    db: Option<Arc<Mutex<MemoryDatabase>>>,
    clipboard_last: Arc<Mutex<String>>,
    /// 关键词 → 工具名
    keyword_map: Vec<(&'static str, &'static str)>,
    search_cache: Arc<Mutex<Option<(String, String)>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let keyword_map = vec![
            ("现在几点","time"),("什么时间","time"),("今天几号","time"),
            ("计算","calc"),("算一下","calc"),
            ("截图","screenshot"),("截屏","screenshot"),
            ("运行代码","code"),("执行代码","code"),
            ("打开浏览器","syscmd"),("打开计算器","syscmd"),("打开记事本","syscmd"),
            ("打开文件管理器","syscmd"),("打开文件夹","syscmd"),("打开微信","syscmd"),
            ("打开 VS Code","syscmd"),("打开编辑器","syscmd"),
            ("打开","syscmd"),
            ("启动","syscmd"),("运行","syscmd"),("关闭","syscmd"),
            ("锁屏","syscmd"),("关机","syscmd"),("重启","syscmd"),("休眠","syscmd"),
            ("剪贴板","clip"),("复制了什么","clip"),
            ("翻译","translate"),("翻译成英语","translate"),
            ("通知","notify"),("弹窗","notify"),
            ("搜索","search"),("搜一下","search"),("帮我搜","search"),("天气","search"),
            ("导出对话","export"),
            ("快捷短语","phrases"),
            ("五星","score"),("好评","score"),
            ("分析情感","emotion"),("我的心情","emotion"),
            ("提醒我","reminder"),("分钟后提醒","reminder"),
            ("查看提醒","reminder_list"),
            ("记住","remember"),("记一下","remember"),("记着","remember"),
            ("搜索对话","search_conv"),("找之前","search_conv"),
        ];
        Self {
            db: None, clipboard_last: Arc::new(Mutex::new(String::new())),
            keyword_map, search_cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<MemoryDatabase>>) { self.db = Some(db); }

    pub fn detect_and_execute(&self, text: &str) -> Option<ToolResult> {
        let t = text.trim();
        let mut matched = None;
        for (kw, tool) in &self.keyword_map {
            if t.contains(*kw) { matched = Some(*tool); break; }
        }
        match matched? {
            "time" => Some(self.get_time()),
            "calc" => self.try_calc(t),
            "screenshot" => Some(self.screenshot()),
            "code" => Some(self.run_code(t)),
            "syscmd" => Some(self.sys_cmd(t)),
            "clip" => Some(self.clip_get()),
            "translate" => Some(self.translate(t)),
            "notify" => Some(self.notify(t)),
            "search" => Some(self.web_search(t)),
            "export" => Some(self.export_conv()),
            "phrases" => Some(self.list_phrases()),
            "score" => Some(self.score_conversation(t)),
            "emotion" => Some(self.analyze_emotion(t)),
            "reminder" => Some(self.set_reminder(t)),
            "reminder_list" => Some(self.list_reminders()),
            "remember" => Some(self.remember(t)),
            "search_conv" => Some(self.search_conversation(t)),
            _ => None,
        }
    }

    fn get_time(&self) -> ToolResult {
        let now = Local::now();
        let wd = ["星期一","星期二","星期三","星期四","星期五","星期六","星期日"];
        let w = wd[now.weekday().num_days_from_monday() as usize];
        let mut o = String::with_capacity(40);
        o.push_str("现在是"); o.push_str(w); o.push(' ');
        o.push_str(&now.format("%Y-%m-%d %H:%M").to_string());
        ToolResult { output: o, should_respond: true, needs_llm: false, llm_prompt: None }
    }

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
        let e = expr.replace('×', "*").replace('÷', "/").replace('^', "**");
        match self.eval(&e) {
            Ok(r) => {
                let s = if r.fract() == 0.0 && r.abs() < 1e15 { format!("{}", r as i64) }
                else { format!("{:.4}", r).trim_end_matches('0').trim_end_matches('.').to_string() };
                let mut o = String::with_capacity(expr.len() + s.len() + 3);
                o.push_str(expr); o.push_str(" = "); o.push_str(&s);
                ToolResult { output: o, should_respond: true, needs_llm: false, llm_prompt: None }
            }
            Err(e) => ToolResult::simple(&format!("计算错误: {}", e)),
        }
    }

    fn eval(&self, e: &str) -> Result<f64, String> {
        let t = self.tokenize(e)?;
        let (r, _) = self.pe(&t, 0)?;
        Ok(r)
    }
    fn tokenize(&self, e: &str) -> Result<Vec<String>, String> {
        let mut t = Vec::with_capacity(e.len() / 2);
        let mut chars = e.chars().peekable();
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_whitespace() { chars.next(); continue; }
            if ch.is_ascii_digit() || ch == '.' {
                let mut n = String::with_capacity(8);
                while let Some(&x) = chars.peek() { if x.is_ascii_digit() || x == '.' { n.push(x); chars.next(); } else { break; } }
                t.push(n);
            } else if "+-*/%()".contains(ch) { t.push(ch.to_string()); chars.next(); }
            else { return Err(format!("Bad: {}", ch)); }
        }
        Ok(t)
    }
    fn pe(&self, t: &[String], pos: usize) -> Result<(f64, usize), String> {
        let (mut r, mut pp) = self.pt(t, pos)?;
        while pp < t.len() { match t[pp].as_str() {
            "+" => { pp+=1; let (v,np)=self.pt(t,pp)?; r+=v; pp=np; }
            "-" => { pp+=1; let (v,np)=self.pt(t,pp)?; r-=v; pp=np; }
            _ => break,
        }} Ok((r, pp))
    }
    fn pt(&self, t: &[String], pos: usize) -> Result<(f64, usize), String> {
        let (mut r, mut pp) = self.pf(t, pos)?;
        while pp < t.len() { match t[pp].as_str() {
            "*" => { pp+=1; let (v,np)=self.pf(t,pp)?; r*=v; pp=np; }
            "/" => { pp+=1; let (v,np)=self.pf(t,pp)?; if v==0.0 { return Err("除以零".into()); } r/=v; pp=np; }
            "%" => { pp+=1; let (v,np)=self.pf(t,pp)?; r%=v; pp=np; }
            _ => break,
        }} Ok((r, pp))
    }
    fn pf(&self, t: &[String], mut pos: usize) -> Result<(f64, usize), String> {
        if pos>=t.len() { return Err("EOF".into()); }
        match t[pos].as_str() {
            "(" => { pos+=1; let (r,np)=self.pe(t,pos)?; if np>=t.len()||t[np]!=")" { return Err("Missing )".into()); } Ok((r,np+1)) }
            "-" => { pos+=1; let (v,np)=self.pf(t,pos)?; Ok((-v,np)) }
            x => Ok((x.parse::<f64>().map_err(|_|format!("Bad: {}",x))?, pos+1)),
        }
    }

    fn screenshot(&self) -> ToolResult {
        let dir = dirs::data_local_dir().unwrap_or_default().join("voice-assistant").join("screenshots");
        let _ = std::fs::create_dir_all(&dir);
        let ts = Local::now().format("%Y%m%d_%H%M%S");
        let path = dir.join(format!("shot_{}.png", ts));
        let p = path.to_str().unwrap_or("");
        let ps = format!("Add-Type -AssemblyName System.Windows.Forms,System.Drawing;$s=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds;$b=New-Object System.Drawing.Bitmap($s.Width,$s.Height);$g=[System.Drawing.Graphics]::FromImage($b);$g.CopyFromScreen($s.Location,[System.Drawing.Point]::Empty,$s.Size);$b.Save('{}');$g.Dispose();$b.Dispose()", p.replace('\\',"\\\\"));
        match std::process::Command::new("powershell").args(["-NoProfile","-Command",&ps]).spawn() {
            Ok(_) => ToolResult::simple(&format!("截图已保存: {}", p)),
            Err(e) => ToolResult::simple(&format!("截图失败: {}", e)),
        }
    }

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
    fn read_clip(&self) -> Result<String,String> {
        let o = std::process::Command::new("powershell").args(["-NoProfile","-Command","Get-Clipboard"]).output().map_err(|e|e.to_string())?;
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
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

    fn translate(&self, t: &str) -> ToolResult {
        let text = if let Some(pos)=t.find("翻译成英语") { t[pos..].trim_start_matches("翻译成英语").trim() }
        else if let Some(pos)=t.find("翻译") { t[pos..].trim_start_matches("翻译").trim() }
        else { t.trim() };
        if text.is_empty() { return ToolResult::simple("你想翻译什么？"); }
        ToolResult::llm("", &format!("将以下文本翻译成英语，只输出翻译结果：\n{}", text))
    }

    fn notify(&self, t: &str) -> ToolResult {
        let raw = t.replace("通知","").replace("提醒我注意","").replace("弹窗","");
        let msg = if raw.trim().is_empty() { "来了一条通知" } else { raw.trim() };
        let ps = format!(r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml("<toast><visual><binding template='ToastGeneric'><text>Mini</text><text>{}</text></binding></visual></toast>")
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Mini").Show($toast)"#, msg.replace('"',"\\\""));
        match std::process::Command::new("powershell").args(["-NoProfile","-Command",&ps]).spawn() {
            Ok(_) => ToolResult::simple(&format!("通知已发送：{}",msg)),
            Err(e) => ToolResult::simple(&format!("通知失败: {}",e)),
        }
    }

    fn web_search(&self, t: &str) -> ToolResult {
        let kws = ["搜索","搜一下","查一下","帮我搜","帮我查","百度","谷歌","天气"];
        let mut q = t.to_string();
        for k in &kws { q = q.replace(k,""); }
        let q = q.trim().to_string();
        if q.is_empty() { return ToolResult::simple("你想搜索什么？"); }
        // 缓存
        { let cache = self.search_cache.lock().unwrap(); if let Some((ref cq,ref cr))=*cache { if *cq==q { return ToolResult::simple(cr); } } }
        match self.search_ddg(&q) {
            Ok(r) => { if r.is_empty() { ToolResult::simple(&format!("没找到「{}」的结果",q)) }
            else { { let mut cache=self.search_cache.lock().unwrap(); *cache=Some((q,r.clone())); } ToolResult::simple(&r) } }
            Err(e) => ToolResult::simple(&format!("搜索失败: {}",e)),
        }
    }
    fn search_ddg(&self, q: &str) -> Result<String,String> {
        let c = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(10)).user_agent("Mozilla/5.0").build().map_err(|e|e.to_string())?;
        let url = format!("https://lite.duckduckgo.com/lite/?q={}",urlencoding::encode(q));
        let html = c.get(&url).send().map_err(|e|e.to_string())?.text().map_err(|e|e.to_string())?;
        let mut res = Vec::with_capacity(5);
        for line in html.lines() { let l=line.trim();
            if l.contains("result-link")||l.contains("result__a") {
                if let Some(s)=l.find("href=\"") { let r=&l[s+6..];
                    if let Some(e)=r.find('"') { let txt=if let Some(ts)=r.find('>') { if let Some(te)=r[ts..].find("</a>") { r[ts+1..ts+te].trim().replace("<b>","").replace("</b>","") } else { String::new() } } else { String::new() };
                        if !txt.is_empty() && res.len()<5 { res.push(txt); }
                    }
                }
            }
        }
        if res.is_empty() { Ok("搜索解析失败".to_string()) }
        else { let mut o=String::with_capacity(200); o.push_str("搜索结果：\n"); for (i,r) in res.iter().enumerate() { o.push_str(&format!("{}. {}\n",i+1,r)); } Ok(o) }
    }

    fn export_conv(&self) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("数据库不可用"); };
        let db=db.lock().unwrap();
        let dir=dirs::data_local_dir().unwrap_or_default().join("voice-assistant").join("exports");
        let _=std::fs::create_dir_all(&dir);
        let ts=Local::now().format("%Y%m%d_%H%M%S");
        let path=dir.join(format!("chat_{}.md",ts));
        let mut content=String::with_capacity(4096);
        content.push_str(&format!("# Mini 对话记录\n\n导出时间：{}\n\n---\n\n",Local::now().format("%Y-%m-%d %H:%M:%S")));
        match db.get_recent_conversations("",10000) {
            Ok(cs)=>{ for (r,t,tm) in &cs { let i=if r=="user"{"🎤"}else{"🤖"}; content.push_str(&format!("**{}** [{}]\n{}\n\n",i,tm,t)); } }
            Err(e)=>return ToolResult::simple(&format!("导出失败: {}",e)),
        }
        match std::fs::write(&path,&content) { Ok(_)=>ToolResult::simple(&format!("已导出: {}",path.display())), Err(e)=>ToolResult::simple(&format!("保存失败: {}",e)) }
    }

    fn set_reminder(&self, t: &str) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("提醒不可用"); };
        let (secs,msg) = if let Some(p)=t.find("分钟后") { let n:String=t[..p].chars().rev().take_while(|c|c.is_ascii_digit()).collect(); let a:i64=n.chars().rev().collect::<String>().parse().unwrap_or(5); let m=if p+3<t.len(){t[p+3..].trim()}else{"提醒事项"}; (a*60,m.to_string()) }
        else if let Some(p)=t.find("小时后") { let n:String=t[..p].chars().rev().take_while(|c|c.is_ascii_digit()).collect(); let a:i64=n.chars().rev().collect::<String>().parse().unwrap_or(1); let m=if p+3<t.len(){t[p+3..].trim()}else{"提醒事项"}; (a*3600,m.to_string()) }
        else { (300,t.to_string()) };
        let at=Local::now()+chrono::Duration::seconds(secs);
        let msg=if msg.len()>50{format!("{}...",&msg[..47])}else{msg};
        let db=db.lock().unwrap();
        match db.create_reminder(&at.naive_local(),&msg) { Ok(_)=>ToolResult::simple(&format!("已设置提醒：{}",msg)), Err(e)=>ToolResult::simple(&format!("设置失败: {}",e)) }
    }
    fn list_reminders(&self) -> ToolResult {
        let Some(ref db)=self.db else { return ToolResult::simple("提醒不可用"); };
        let db=db.lock().unwrap();
        match db.get_pending_reminders() { Ok(rs)=>if rs.is_empty(){ToolResult::simple("没有提醒")}else{let mut l=Vec::with_capacity(rs.len()+1); l.push(format!("{}个提醒：",rs.len())); for r in rs.iter().take(5){l.push(format!("- {}: {}",r.1,r.2));} ToolResult::simple(&l.join("\n"))}, Err(e)=>ToolResult::simple(&format!("查询失败: {}",e)) }
    }

    fn remember(&self, t: &str) -> ToolResult {
        let kws = ["记住","记一下","记着","以后记得"];
        let mut c=String::new();
        for k in &kws { if let Some(p)=t.find(k){c=t[p+k.len()..].trim().to_string();break;} }
        if c.is_empty() { return ToolResult::simple("你想让我记住什么？"); }
        let Some(ref db)=self.db else { return ToolResult::simple("记忆不可用"); };
        let db=db.lock().unwrap();
        match db.save_memory(&c,"preference",Some("user"),None,0.5) { Ok(_)=>ToolResult::simple(&format!("好的，记住了：{}",c)), Err(e)=>ToolResult::simple(&format!("保存失败: {}",e)) }
    }

    fn search_conversation(&self, t: &str) -> ToolResult {
        let kws = ["搜索对话","查找对话","搜历史","找之前","搜索","查找"];
        let mut q=t.to_string(); for k in &kws{q=q.replace(k,"");}
        let q=q.trim();
        if q.is_empty() { return ToolResult::simple("你想搜索什么对话？"); }
        let Some(ref db)=self.db else { return ToolResult::simple("数据库不可用"); };
        let db=db.lock().unwrap();
        match db.search_memories_fts(q,5) { Ok(results)=>{if results.is_empty(){ToolResult::simple(&format!("没找到关于「{}」的记录",q))}else{let mut l=Vec::with_capacity(results.len()+1); l.push(format!("找到{}条记录：",results.len())); for (_,c,cat,_) in &results{l.push(format!("- [{}] {}",cat,c));} ToolResult::simple(&l.join("\n"))}}, Err(e)=>ToolResult::simple(&format!("搜索失败: {}",e)) }
    }

    fn run_code(&self, t: &str) -> ToolResult {
        let code=t.replace("运行代码","").replace("执行代码","").replace("跑代码","").replace("run code","").trim().to_string();
        if code.is_empty() { return ToolResult::simple("你想运行什么代码？"); }
        if code.contains("import os")||code.contains("subprocess")||code.contains("exec(")||code.contains("__import__") { return ToolResult::simple("⚠️ 安全限制：不允许执行危险代码"); }
        match std::process::Command::new("python").args(["-c",&code]).output() {
            Ok(o)=>{let s=String::from_utf8_lossy(&o.stdout).to_string();let e=String::from_utf8_lossy(&o.stderr).to_string();if o.status.success(){let r=if s.len()>500{format!("{}...",&s[..500])}else{s};ToolResult::simple(&format!("✅ 执行结果：\n{}",r))}else{ToolResult::simple(&format!("❌ 执行错误：\n{}",e))}},
            Err(e)=>ToolResult::simple(&format!("执行失败: {}",e)),
        }
    }

    fn analyze_emotion(&self, t: &str) -> ToolResult {
        let raw=t.replace("分析情感","").replace("你的情绪","").replace("我的心情","").replace("情绪分析","");
        let text=raw.trim();
        if text.is_empty() { return ToolResult::simple("你想分析什么内容的情感？"); }
        let (emotion,score) = if text.contains("开心")||text.contains("高兴")||text.contains("快乐"){("😊 开心",0.9)}
        else if text.contains("难过")||text.contains("伤心"){("😢 难过",0.8)}
        else if text.contains("生气")||text.contains("愤怒"){("😠 生气",0.85)}
        else if text.contains("担心")||text.contains("焦虑"){("😰 焦虑",0.75)}
        else if text.contains("感谢")||text.contains("谢谢"){("🙏 感激",0.85)}
        else {("😐 平静",0.5)};
        let advice=match emotion{"😊 开心"=>"保持好心情！","😢 难过"=>"需要聊聊吗？","😠 生气"=>"深呼吸。","😰 焦虑"=>"别担心。","🙏 感激"=>"不客气！",_=>"有什么我能帮你的？"};
        ToolResult::simple(&format!("情感：{}\n置信度：{:.0}%\n建议：{}",emotion,score*100.0,advice))
    }

    fn list_phrases(&self) -> ToolResult {
        let phrases=[("天气","搜索今天的天气"),("新闻","搜索今天的新闻"),("时间","现在几点了"),("截图","截图"),("导出","导出对话")];
        let mut l=Vec::with_capacity(phrases.len()+1); l.push("快捷短语：".to_string());
        for (i,(n,p)) in phrases.iter().enumerate(){l.push(format!("{}. {} → \"{}\"",i+1,n,p));}
        ToolResult::simple(&l.join("\n"))
    }

    fn score_conversation(&self, t: &str) -> ToolResult {
        let score=if t.contains("五星")||t.contains("好评")||t.contains("👍"){5}else if t.contains("四星"){4}else if t.contains("三星")||t.contains("一般"){3}else if t.contains("二星")||t.contains("差评"){2}else if t.contains("一星")||t.contains("👎"){1}else{3};
        if let Some(ref db)=self.db{let db=db.lock().unwrap();let _=db.save_memory(&format!("用户评分: {}星",score),"feedback",Some("user"),None,0.4);}
        let emoji=match score{1=>"😢",2=>"😐",3=>"🙂",4=>"😊",5=>"🤩",_=>"🙂"};
        ToolResult::simple(&format!("{} 感谢评分！{}星。",emoji,score))
    }
}
