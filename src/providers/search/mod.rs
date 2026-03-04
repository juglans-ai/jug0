// src/providers/search/mod.rs
pub mod tavily;

use crate::services::search::SearchResponse;
use async_trait::async_trait;

#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str) -> anyhow::Result<SearchResponse>;
}
