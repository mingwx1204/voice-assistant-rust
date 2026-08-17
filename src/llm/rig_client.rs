/// llm/rig_client.rs — LLM 推理客户端
/// =======================================
/// 通过 HTTP 调用本地 llama.cpp 的 OpenAI 兼容 API。
/// 支持普通聊天和流式响应。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;

/// LLM 客户端
pub struct LlmClient {
    base_url: String,
    model: String,
    api_key: String,
    max_tokens: u32,
    temperature: f32,
    http_client: reqwest::blocking::Client,
    available: bool,
}

/// OpenAI 兼容 API 请求
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI 兼容 API 响应
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// 流式响应的 chunk
#[derive(Deserialize)]
pub struct StreamChunk {
    pub choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
pub struct StreamChoice {
    pub delta: Option<StreamDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct StreamDelta {
    pub content: Option<String>,
    #[allow(dead_code)]
    pub role: Option<String>,
}

/// Models 列表响应
#[derive(Deserialize)]
struct ModelsResponse {
    #[allow(dead_code)]
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    #[allow(dead_code)]
    id: String,
}

impl LlmClient {
    /// 创建 LLM 客户端
    pub fn new(config: &LlmConfig) -> Result<Self> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        let mut client = Self {
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            http_client,
            available: false,
        };

        client.check_connection();
        Ok(client)
    }

    /// 检查连接
    fn check_connection(&mut self) {
        match self.http_client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(models) = resp.json::<ModelsResponse>() {
                    let names: Vec<&str> = models.data.iter().map(|m| m.id.as_str()).collect();
                    tracing::info!("LLM connected (models: {})", names.join(", "));
                } else {
                    tracing::info!("LLM connected to {}", self.base_url);
                }
                self.available = true;
            }
            Ok(resp) => {
                tracing::warn!("LLM returned status: {}", resp.status());
                self.available = false;
            }
            Err(e) => {
                tracing::warn!("LLM connection failed: {}", e);
                self.available = false;
            }
        }
    }

    /// 构建消息列表
    fn build_messages(user_message: &str, system_message: Option<&str>, history: &[ChatMessage]) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        if let Some(system) = system_message {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
            });
        }

        // 添加历史对话
        messages.extend_from_slice(history);

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        messages
    }

    /// 聊天补全 (同步，非流式)
    pub fn chat_sync(
        &mut self,
        user_message: &str,
        system_message: Option<&str>,
        history: &[ChatMessage],
    ) -> Result<String> {
        if !self.available {
            self.check_connection();
            if !self.available {
                return Ok("抱歉，模型服务不可用。请检查 llama-server 是否运行。".to_string());
            }
        }

        let messages = Self::build_messages(user_message, system_message, history);

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: false,
        };

        let start = std::time::Instant::now();

        let response = self.http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .context("LLM API call failed")?;

        let elapsed = start.elapsed();

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("LLM API error: {} - {}", status, body);
        }

        let chat_response: ChatResponse = response
            .json()
            .context("Failed to parse LLM response")?;

        let reply = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!(
            "LLM reply ({:.1}s): {}",
            elapsed.as_secs_f32(),
            &reply[..reply.len().min(50)]
        );

        Ok(reply)
    }

    /// 流式聊天补全 — 通过回调逐 token 返回
    ///
    /// `on_token` 回调在每个 token 到达时被调用。
    /// 返回完整的回复文本。
    pub fn chat_stream(
        &mut self,
        user_message: &str,
        system_message: Option<&str>,
        history: &[ChatMessage],
        mut on_token: impl FnMut(&str),
    ) -> Result<String> {
        if !self.available {
            self.check_connection();
            if !self.available {
                let msg = "抱歉，模型服务不可用。请检查 llama-server 是否运行。".to_string();
                on_token(&msg);
                return Ok(msg);
            }
        }

        let messages = Self::build_messages(user_message, system_message, history);

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: true,
        };

        let start = std::time::Instant::now();

        let response = self.http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .context("LLM stream API call failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("LLM API error: {} - {}", status, body);
        }

        let mut full_reply = String::new();
        let mut buffer = String::new();

        // 逐行读取 SSE 流
        let reader = response;
        let text = reader.text().context("Failed to read stream")?;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                match serde_json::from_str::<StreamChunk>(data) {
                    Ok(chunk) => {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    buffer.push_str(content);
                                    on_token(content);
                                }
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
        }

        full_reply = buffer;

        let elapsed = start.elapsed();
        tracing::info!(
            "LLM stream ({:.1}s): {}",
            elapsed.as_secs_f32(),
            &full_reply[..full_reply.len().min(50)]
        );

        Ok(full_reply)
    }

    /// 检查是否可用
    pub fn is_available(&self) -> bool {
        self.available
    }
}
