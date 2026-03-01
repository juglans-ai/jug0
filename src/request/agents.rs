// src/request/agents.rs
//
// Agent-related request types

use serde::{Deserialize, Deserializer};
use uuid::Uuid;

/// Deserializer for double-Option fields to distinguish "absent" vs "explicit null":
///   - absent from JSON → None (don't touch the field)
///   - explicit null     → Some(None) (clear the field)
///   - present value     → Some(Some(value)) (set the field)
pub fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

/// Create agent request
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt_id: Uuid,

    #[serde(default = "default_model")]
    pub default_model: String,
    pub allowed_models: Option<Vec<String>>,

    pub skills: Option<Vec<String>>,
    pub mcp_config: Option<serde_json::Value>,
    pub temperature: Option<f64>,

    /// Associated workflow ID (optional)
    pub workflow_id: Option<Uuid>,

    /// Is public
    pub is_public: Option<bool>,

    /// @username for this agent (auto-registers handle)
    pub username: Option<String>,

    /// Avatar URL
    pub avatar: Option<String>,
}

/// Update agent request
#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt_id: Option<Uuid>,
    pub default_model: Option<String>,
    pub allowed_models: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub mcp_config: Option<serde_json::Value>,
    pub temperature: Option<f64>,
    /// Associated workflow ID:
    ///   - absent from JSON → None (don't touch)
    ///   - explicit null     → Some(None) (clear association)
    ///   - "uuid-string"     → Some(Some(uuid)) (set association)
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub workflow_id: Option<Option<Uuid>>,
    /// Is public
    pub is_public: Option<bool>,
    /// @username for this agent (set to null to remove handle)
    pub username: Option<String>,

    /// Avatar URL
    pub avatar: Option<String>,
}
