// src/providers/llm/factory.rs
use super::{
    anthropic::AnthropicProvider, chatgpt::ChatGPTProvider, claude_code::ClaudeCodeProvider,
    deepseek::DeepSeekProvider, gemini::GeminiProvider, qwen::QwenProvider, LlmProvider,
};
use std::env;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProviderFactory {
    anthropic: Arc<AnthropicProvider>,
    chatgpt: Arc<ChatGPTProvider>,
    claude_code: Arc<ClaudeCodeProvider>,
    deepseek: Arc<DeepSeekProvider>,
    gemini: Arc<GeminiProvider>,
    qwen: Arc<QwenProvider>,
}

impl ProviderFactory {
    pub fn new() -> Self {
        Self {
            anthropic: Arc::new(AnthropicProvider::new()),
            chatgpt: Arc::new(ChatGPTProvider::new()),
            claude_code: Arc::new(ClaudeCodeProvider::new()),
            deepseek: Arc::new(DeepSeekProvider::new()),
            gemini: Arc::new(GeminiProvider::new()),
            qwen: Arc::new(QwenProvider::new()),
        }
    }

    pub fn get_provider(&self, model: &str) -> Arc<dyn LlmProvider> {
        let m = model.to_lowercase();
        let default_provider = env::var("DEFAULT_LLM_PROVIDER")
            .unwrap_or_default()
            .to_lowercase();

        // claude-code must be checked before claude (more specific match first)
        if m.contains("claude-code") || (m == "default" && default_provider == "claude-code") {
            return self.claude_code.clone();
        }

        if m.contains("claude") || (m == "default" && default_provider == "anthropic") {
            return self.anthropic.clone();
        }

        if m.contains("qwen") || (m == "default" && default_provider == "qwen") {
            return self.qwen.clone();
        }

        if m.contains("gemini") {
            return self.gemini.clone();
        }

        if m.contains("deepseek") {
            return self.deepseek.clone();
        }

        self.chatgpt.clone()
    }
}
