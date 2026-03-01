// src/request/mod.rs
//
// Unified request types for all API endpoints

pub mod agents;
pub mod prompts;
pub mod workflows;
pub mod chats;
pub mod auth;
pub mod users;
pub mod api_keys;
pub mod organizations;
pub mod models;
pub mod memories;
pub mod embeddings;

// Re-export all types at module root for convenience
pub use agents::{CreateAgentRequest, UpdateAgentRequest};
pub use prompts::{CreatePromptRequest, UpdatePromptRequest, PromptFilter, RenderPromptRequest};
pub use workflows::{CreateWorkflowRequest, UpdateWorkflowRequest, ExecuteWorkflowRequest};
pub use chats::{
    ChatRequest, ChatIdInput, StopRequest, ListChatsQuery,
    MessagePart, AgentConfig, ToolResultPayload, ToolResultRequest,
    CreateMessageRequest, ContextQuery, UpdateMessageRequest,
    RegenerateRequest, BranchRequest,
};
pub use auth::{LoginRequest, RegisterRequest};
pub use users::{SyncUserRequest, BatchSyncRequest};
pub use api_keys::CreateApiKeyRequest;
pub use organizations::SetPublicKeyRequest;
pub use models::ModelsQuery;
pub use memories::{ListMemoryQuery, SearchMemoryRequest};
pub use embeddings::EmbeddingRequest;
