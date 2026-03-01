// src/providers/llm/factory.rs
use super::{LlmProvider, chatgpt::ChatGPTProvider, deepseek::DeepSeekProvider, gemini::GeminiProvider, qwen::QwenProvider};
use std::sync::Arc;
use std::env;

#[derive(Clone)]
pub struct ProviderFactory {
    chatgpt: Arc<ChatGPTProvider>,
    deepseek: Arc<DeepSeekProvider>,
    gemini: Arc<GeminiProvider>,
    qwen: Arc<QwenProvider>,
}

impl ProviderFactory {
    pub fn new() -> Self {
        Self {
            chatgpt: Arc::new(ChatGPTProvider::new()),
            deepseek: Arc::new(DeepSeekProvider::new()),
            gemini: Arc::new(GeminiProvider::new()),
            qwen: Arc::new(QwenProvider::new()),
        }
    }

    pub fn get_provider(&self, model: &str) -> Arc<dyn LlmProvider> {
        let m = model.to_lowercase();
        let default_provider = env::var("DEFAULT_LLM_PROVIDER").unwrap_or_default().to_lowercase();

        // 逻辑：如果显式指定了模型名包含 qwen，或者全局默认是 qwen 且没传特定模型
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