// src/handlers/search.rs
//
// POST /api/search — Web search via Tavily API

use axum::{Extension, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::auth::AuthUser;
use crate::errors::AppError;
use crate::services::search::SearchResponse;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
}

/// POST /api/search
/// Requires authentication. Proxies search to Tavily API.
pub async fn web_search(
    Extension(state): Extension<Arc<AppState>>,
    _user: AuthUser,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, AppError> {
    if req.query.trim().is_empty() {
        return Err(AppError::BadRequest("Search query cannot be empty".to_string()));
    }

    let result = state.search_service.search(&req.query).await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(result))
}
