// src/response/mod.rs
//
// Unified response types for all API endpoints

pub mod common;
pub mod agents;
pub mod prompts;
pub mod workflows;
pub mod chats;
pub mod auth;
pub mod users;
pub mod api_keys;
pub mod organizations;
pub mod models;
pub mod embeddings;
pub mod usage;
pub mod resources;

// Re-export common types at module root for convenience
pub use common::{OwnerInfo, PublicUserProfile, SuccessResponse};
pub use agents::{AgentWithOwner, AgentDetailResponse};
pub use prompts::{PromptWithOwner, RenderPromptResponse};
pub use workflows::{WorkflowWithOwner, ExecuteWorkflowResponse};
pub use chats::{ChatSyncResponse, MessageResponse, ContextResponse, BranchResponse, StreamEvent};
pub use auth::{AuthResponse, UserDto, MeResponse};
pub use users::{SyncUserResponse, BatchSyncResponse};
pub use api_keys::CreateApiKeyResponse;
pub use organizations::{SetPublicKeyResponse, OrgInfoResponse};
pub use models::ModelsResponse;
pub use embeddings::EmbeddingResponse;
pub use usage::{UsageStats, ModelUsage};
pub use resources::{ResourceResponse, ResourcePrompt, ResourceAgent, ResourceWorkflow};
