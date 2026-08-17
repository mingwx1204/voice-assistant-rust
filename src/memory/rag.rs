/// memory/rag.rs — RAG 知识库
/// ============================
/// 加载本地文本文件，基于内容回答问题。

use anyhow::Result;
use std::path::{Path, PathBuf};

/// 知识库文档
#[derive(Debug, Clone)]
pub struct Document {
    pub path: PathBuf,
    pub content: String,
    pub chunks: Vec<String>,
}

/// RAG 知识库
pub struct KnowledgeBase {
    documents: Vec<Document>,
    data_dir: PathBuf,
}

impl KnowledgeBase {
    /// 创建知识库
    pub fn new(data_dir: &Path) -> Result<Self> {
        let kb_dir = data_dir.join("knowledge");
        std::fs::create_dir_all(&kb_dir)?;

        let mut kb = Self {
            documents: Vec::new(),
            data_dir: kb_dir,
        };

        kb.load_documents()?;
        Ok(kb)
    }

    /// 加载所有文档
    fn load_documents(&mut self) -> Result<()> {
        self.documents.clear();

        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                match ext {
                    "txt" | "md" | "csv" => {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let chunks = self.chunk_text(&content);
                                tracing::info!("Loaded document: {:?} ({} chunks)", path.file_name(), chunks.len());
                                self.documents.push(Document { path, content, chunks });
                            }
                            Err(e) => {
                                tracing::warn!("Failed to read {:?}: {}", path, e);
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        tracing::info!("Knowledge base: {} documents loaded", self.documents.len());
        Ok(())
    }

    /// 文本分块（按段落/句子）
    fn chunk_text(&self, text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                if !current.is_empty() {
                    chunks.push(std::mem::take(&mut current));
                }
            } else {
                if !current.is_empty() {
                    current.push('\n');
                }
                current.push_str(line);

                // 如果块够大，就分割
                if current.len() > 500 {
                    chunks.push(std::mem::take(&mut current));
                }
            }
        }

        if !current.is_empty() {
            chunks.push(current);
        }

        chunks
    }

    /// 搜索相关文档片段
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(String, f32)> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<(String, f32)> = Vec::new();

        for doc in &self.documents {
            for chunk in &doc.chunks {
                let chunk_lower = chunk.to_lowercase();

                // 简单的关键词匹配评分
                let mut score = 0.0f32;
                for word in &query_words {
                    if chunk_lower.contains(word) {
                        score += 1.0;
                    }
                }

                // 检查完整查询是否出现
                if chunk_lower.contains(&query_lower) {
                    score += 5.0;
                }

                if score > 0.0 {
                    results.push((chunk.clone(), score));
                }
            }
        }

        // 按分数排序，取 top_k
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(top_k);

        results
    }

    /// 添加文档
    pub fn add_document(&mut self, name: &str, content: &str) -> Result<()> {
        let path = self.data_dir.join(format!("{}.txt", name));
        std::fs::write(&path, content)?;

        let chunks = self.chunk_text(content);
        self.documents.push(Document {
            path,
            content: content.to_string(),
            chunks,
        });

        Ok(())
    }

    /// 获取文档数量
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// 获取总块数
    pub fn chunk_count(&self) -> usize {
        self.documents.iter().map(|d| d.chunks.len()).sum()
    }
}
