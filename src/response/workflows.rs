// src/response/workflows.rs
//
// Workflow-related response types

use super::common::OwnerInfo;
use crate::entities::workflows;
use serde::Serialize;
use uuid::Uuid;

/// Workflow with owner information for list responses
#[derive(Debug, Serialize)]
pub struct WorkflowWithOwner {
    #[serde(flatten)]
    pub workflow: workflows::Model,
    pub owner: Option<OwnerInfo>,
    pub url: Option<String>,
}

/// Workflow execution response
#[derive(Debug, Serialize)]
pub struct ExecuteWorkflowResponse {
    pub workflow_id: Uuid,
    pub status: String,
    pub message: String,
    pub result: Option<serde_json::Value>,
}
