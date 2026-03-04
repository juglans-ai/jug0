// src/entities/agents.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub slug: String,

    pub org_id: Option<String>, // 新增
    pub user_id: Option<Uuid>,  // 变更 UUID

    pub name: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub system_prompt_id: Option<Uuid>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub allowed_models: Option<serde_json::Value>,
    pub default_model: Option<String>,

    pub temperature: Option<f64>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub mcp_config: Option<serde_json::Value>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub skills: Option<serde_json::Value>,

    pub fork_from_id: Option<Uuid>,

    /// 关联的 workflow (一对一，可选)
    pub workflow_id: Option<Uuid>,

    /// 是否公开 (类似 GitHub public repo)
    pub is_public: Option<bool>,

    /// @username for this agent (auto-registers handle)
    pub username: Option<String>,

    /// Avatar URL (uploaded image or external URL)
    pub avatar: Option<String>,

    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::prompts::Entity",
        from = "Column::SystemPromptId",
        to = "super::prompts::Column::Id"
    )]
    SystemPrompt,

    #[sea_orm(belongs_to = "Entity", from = "Column::ForkFromId", to = "Column::Id")]
    ForkFrom,

    #[sea_orm(
        belongs_to = "super::workflows::Entity",
        from = "Column::WorkflowId",
        to = "super::workflows::Column::Id"
    )]
    Workflow,
}

impl Related<super::prompts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SystemPrompt.def()
    }
}

impl Related<Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ForkFrom.def()
    }
}

impl Related<super::workflows::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
