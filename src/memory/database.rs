/// memory/database.rs — SQLite 记忆数据库
/// ==========================================
/// 负责数据库初始化、表结构创建、CRUD 操作。
use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};
use std::path::Path;

/// 记忆条目: (id, content, category, importance, access_count)
pub type MemoryRow = (i64, String, String, f32, i32);

/// 记忆数据库
pub struct MemoryDatabase {
    conn: Connection,
}

impl MemoryDatabase {
    /// 创建或打开数据库
    pub fn new(db_path: &Path) -> Result<Self> {
        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path).context("Failed to open database")?;

        // 启用 WAL 模式
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let mut db = Self { conn };
        db.create_tables()?;
        db.create_fts_index()?;

        tracing::info!("Database opened: {:?}", db_path);
        Ok(db)
    }

    /// 创建表结构
    fn create_tables(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                content TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                category TEXT DEFAULT 'fact',
                source TEXT,
                embedding BLOB,
                importance REAL DEFAULT 0.5,
                access_count INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                remind_at TIMESTAMP NOT NULL,
                message TEXT NOT NULL,
                is_fired INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_conv_session ON conversations(session_id);
            CREATE INDEX IF NOT EXISTS idx_mem_category ON memories(category);
            CREATE INDEX IF NOT EXISTS idx_remind_at ON reminders(remind_at, is_fired);
            ",
        )?;
        Ok(())
    }

    /// 创建 FTS5 索引
    fn create_fts_index(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                content, category, source, content=memories, content_rowid=id
            );
            ",
        )?;
        Ok(())
    }

    // === 对话操作 ===

    /// 保存对话
    pub fn save_conversation(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO conversations (session_id, role, content) VALUES (?, ?, ?)",
            params![session_id, role, content],
        )?;
        Ok(())
    }

    /// 获取最近的对话
    pub fn get_recent_conversations(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content, created_at FROM conversations
             WHERE session_id = ? ORDER BY id DESC LIMIT ?",
        )?;

        let rows = stmt
            .query_map(params![session_id, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // 反转顺序（从旧到新）
        Ok(rows.into_iter().rev().collect())
    }

    /// 统计对话数
    pub fn count_conversations(&self, session_id: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE session_id = ? AND role = 'user'",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // === 记忆操作 ===

    /// 保存记忆
    pub fn save_memory(
        &self,
        content: &str,
        category: &str,
        source: Option<&str>,
        embedding: Option<&[u8]>,
        importance: f32,
    ) -> Result<i64> {
        let _row_id = self.conn.execute(
            "INSERT INTO memories (content, category, source, embedding, importance) VALUES (?, ?, ?, ?, ?)",
            params![content, category, source, embedding, importance],
        )?;

        let id = self.conn.last_insert_rowid();

        // 更新 FTS 索引
        let _ = self.conn.execute(
            "INSERT INTO memories_fts(rowid, content, category, source) VALUES (?, ?, ?, ?)",
            params![id, content, category, source.unwrap_or("")],
        );

        Ok(id)
    }

    /// 搜索记忆 (FTS5)
    pub fn search_memories_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(i64, String, String, f32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.content, m.category, m.importance
             FROM memories_fts fts
             JOIN memories m ON m.id = fts.rowid
             WHERE memories_fts MATCH ?
             ORDER BY rank
             LIMIT ?",
        )?;

        let rows = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f32>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// 获取所有记忆
    pub fn get_all_memories(&self) -> Result<Vec<MemoryRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, category, importance, access_count FROM memories ORDER BY id",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f32>(3)?,
                    row.get::<_, i32>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// 更新记忆访问次数
    pub fn update_memory_access(&self, memory_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE memories SET access_count = access_count + 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![memory_id],
        )?;
        Ok(())
    }

    /// 删除记忆
    pub fn delete_memory(&self, memory_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM memories WHERE id = ?", params![memory_id])?;
        Ok(())
    }

    // === 提醒操作 ===

    /// 创建提醒
    pub fn create_reminder(&self, remind_at: &chrono::NaiveDateTime, message: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO reminders (remind_at, message) VALUES (?, ?)",
            params![remind_at.to_string(), message],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 获取待处理的提醒
    pub fn get_pending_reminders(&self) -> Result<Vec<(i64, String, String)>> {
        let now = Local::now().naive_local().to_string();
        let mut stmt = self.conn.prepare(
            "SELECT id, remind_at, message FROM reminders
             WHERE is_fired = 0 AND remind_at <= ?
             ORDER BY remind_at",
        )?;

        let rows = stmt
            .query_map(params![now], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// 标记提醒已触发
    pub fn fire_reminder(&self, reminder_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE reminders SET is_fired = 1 WHERE id = ?",
            params![reminder_id],
        )?;
        Ok(())
    }

    /// 获取待处理提醒数量
    pub fn get_pending_reminder_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reminders WHERE is_fired = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // === 统计 ===

    /// 获取统计信息
    pub fn get_stats(&self) -> Result<MemoryStats> {
        let conversations: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0))?;

        let memories: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;

        let pending_reminders = self.get_pending_reminder_count()?;

        Ok(MemoryStats {
            conversations: conversations as usize,
            memories: memories as usize,
            pending_reminders,
        })
    }
}

/// 记忆统计信息
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub conversations: usize,
    pub memories: usize,
    pub pending_reminders: usize,
}
