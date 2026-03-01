// src/providers/llm/mod.rs
pub mod chatgpt;
pub mod deepseek;
pub mod gemini;
pub mod openai;
pub mod qwen;
pub mod factory;

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use crate::entities::messages;
use anyhow::Result;
use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ToolCallChunk {
    pub index: i32,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
    pub signature: Option<String>,
}

/// Token usage statistics from LLM providers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone)]
pub struct ChatStreamChunk {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallChunk>,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream_chat(
        &self,
        model: &str,
        system_prompt: Option<String>,
        history: Vec<messages::Model>,
        tools: Option<Vec<Value>>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>>;
}
