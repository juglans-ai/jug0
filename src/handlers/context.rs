// src/handlers/context.rs
use axum::{
    extract::{Extension, Path},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, ModelTrait,
    QueryFilter, QueryOrder, Set, Statement,
};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::handlers::chat::resolve_chat_id_strict;
use crate::{
    entities::{chats, messages},
    errors::AppError,
    AppState,
};

// GET /api/chat/:id (supports UUID or @handle)
pub async fn get_history(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthUser,
    Path(id_or_handle): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Resolve chat_id (UUID or @handle)
    let chat_id = resolve_chat_id_strict(&state.db, &user.org_id, user.id, &id_or_handle).await?;

    // 1. 检查 Chat 是否存在且属于当前用户
    let chat = chats::Entity::find_by_id(chat_id)
        .filter(chats::Column::UserId.eq(user.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Chat {} not found or access denied", chat_id))
        })?;

    // 2. 获取消息列表
    let history = messages::Entity::find()
        .filter(messages::Column::ChatId.eq(chat_id))
        .order_by_asc(messages::Column::CreatedAt)
        .all(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "chat": chat,
        "messages": history
    })))
}

// DELETE /api/chat/:id (supports UUID or @handle)
pub async fn delete_chat(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthUser,
    Path(id_or_handle): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Resolve chat_id (UUID or @handle)
    let chat_id = resolve_chat_id_strict(&state.db, &user.org_id, user.id, &id_or_handle).await?;

    // 1. 检查是否存在且属于当前用户
    let chat = chats::Entity::find_by_id(chat_id)
        .filter(chats::Column::UserId.eq(user.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Chat {} not found or access denied", chat_id))
        })?;

    // 2. 删除 (先删消息，再删会话)
    messages::Entity::delete_many()
        .filter(messages::Column::ChatId.eq(chat_id))
        .exec(&state.db)
        .await?;

    chat.delete(&state.db).await?;

    Ok(Json(
        serde_json::json!({ "status": "deleted", "id": chat_id }),
    ))
}

// POST /api/chat/:id/clear — clear old messages, keep current turn
pub async fn clear_chat_history(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthUser,
    Path(id_or_handle): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let chat_id = resolve_chat_id_strict(&state.db, &user.org_id, user.id, &id_or_handle).await?;

    let _chat = chats::Entity::find_by_id(chat_id)
        .filter(chats::Column::UserId.eq(user.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Chat {} not found or access denied", chat_id))
        })?;

    // 找到最后一条 user message 的 message_id，删除之前的所有消息
    let last_user_msg = messages::Entity::find()
        .filter(messages::Column::ChatId.eq(chat_id))
        .filter(messages::Column::Role.eq("user"))
        .order_by_desc(messages::Column::MessageId)
        .one(&state.db)
        .await?;

    let deleted_count = if let Some(last_user) = last_user_msg {
        let result = state
            .db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "DELETE FROM messages WHERE chat_id = $1 AND message_id < $2",
                [chat_id.into(), last_user.message_id.into()],
            ))
            .await?;
        result.rows_affected()
    } else {
        0
    };

    Ok(Json(serde_json::json!({
        "status": "cleared",
        "id": chat_id,
        "deleted_count": deleted_count
    })))
}

// DELETE /api/chat/:id/messages — clear all messages, keep chat
pub async fn clear_chat_messages(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthUser,
    Path(id_or_handle): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let chat_id = resolve_chat_id_strict(&state.db, &user.org_id, user.id, &id_or_handle).await?;

    let chat = chats::Entity::find_by_id(chat_id)
        .filter(chats::Column::UserId.eq(user.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Chat {} not found or access denied", chat_id))
        })?;

    // 只删消息，保留 chat 记录
    let result = messages::Entity::delete_many()
        .filter(messages::Column::ChatId.eq(chat_id))
        .exec(&state.db)
        .await?;

    // 重置 last_message_id
    let mut active: chats::ActiveModel = chat.into();
    active.last_message_id = Set(Some(0));
    active.update(&state.db).await?;

    Ok(Json(serde_json::json!({
        "status": "cleared",
        "id": chat_id,
        "deleted_count": result.rows_affected
    })))
}
