// src/response/models.rs
//
// Model list response types

use serde::Serialize;
use crate::services::models::{ModelInfo, ProviderStatus};

/// Models list response
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
    pub providers: Vec<ProviderStatus>,
}
