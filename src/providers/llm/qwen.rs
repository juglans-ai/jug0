// src/providers/qwen.rs
use super::{ChatStreamChunk, LlmProvider, TokenUsage, ToolCallChunk};
use crate::entities::messages;
use crate::handlers::chat::MessagePart;
use anyhow::Result;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::pin::Pin;
use std::time::Duration;

pub struct QwenProvider {
    client: Client,
    api_key: String,
}

#[derive(Serialize)]
struct QwenChatRequest {
    model: String,
    input: QwenChatInput,
    parameters: QwenChatParameters,
}

#[derive(Serialize)]
struct QwenChatInput {
    messages: Vec<QwenMessage>,
}

#[derive(Serialize, Deserialize)]
struct QwenMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct QwenChatParameters {
    result_format: String, // 设为 "message"
    incremental_output: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct QwenResponseChunk {
    output: QwenOutput,
    usage: Option<QwenUsage>,
}

#[derive(Deserialize)]
struct QwenUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[derive(Deserialize)]
struct QwenOutput {
    choices: Vec<QwenChoice>,
}

#[derive(Deserialize)]
struct QwenChoice {
    message: QwenMessage,
    // finish_reason: String,
}

impl QwenProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();
        Self {
            client,
            api_key: env::var("QWEN_API_KEY").expect("QWEN_API_KEY not set"),
        }
    }
}

#[async_trait]
impl LlmProvider for QwenProvider {
    async fn stream_chat(
        &self,
        model: &str,
        system_prompt: Option<String>,
        history: Vec<messages::Model>,
        _tools: Option<Vec<serde_json::Value>>, // 原生封装暂不处理复杂工具调用逻辑
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>> {
        let mut qwen_messages = Vec::new();

        if let Some(sp) = system_prompt {
            qwen_messages.push(QwenMessage {
                role: "system".to_string(),
                content: sp,
            });
        }

        for msg in history {
            let content = if let Ok(parts) = serde_json::from_value::<Vec<MessagePart>>(msg.parts) {
                parts
                    .iter()
                    .filter_map(|p| p.content.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::new()
            };

            qwen_messages.push(QwenMessage {
                role: msg.role,
                content,
            });
        }

        let request = QwenChatRequest {
            model: model.to_string(),
            input: QwenChatInput {
                messages: qwen_messages,
            },
            parameters: QwenChatParameters {
                result_format: "message".to_string(),
                incremental_output: true, // 关键：开启增量输出
                temperature: Some(0.7),
            },
        };

        let response = self
            .client
            .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-DashScope-SSE", "enable") // 关键：开启 SSE
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let txt = response.text().await?;
            return Err(anyhow::anyhow!("DashScope Chat Error: {}", txt));
        }

        let stream = response.bytes_stream().eventsource();
        let mapped = stream.map(|event_res| {
            let event = event_res.map_err(|e| anyhow::anyhow!("SSE Error: {}", e))?;

            // DashScope SSE 结束后通常不发送 [DONE]，直接关闭连接
            let chunk: QwenResponseChunk = serde_json::from_str(&event.data)?;
            let text = chunk
                .output
                .choices
                .first()
                .map(|c| c.message.content.clone());

            // Extract usage from response
            let usage = chunk.usage.map(|u| TokenUsage {
                input_tokens: u.input_tokens.unwrap_or(0),
                output_tokens: u.output_tokens.unwrap_or(0),
                total_tokens: u.total_tokens.unwrap_or(0),
            });

            Ok(ChatStreamChunk {
                content: text,
                tool_calls: vec![],
                usage,
                finish_reason: None,
            })
        });

        Ok(Box::pin(mapped))
    }
}
