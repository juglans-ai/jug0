// src/handlers/users.rs
//
// Internal user sync handler for juglans-api integration

use axum::{
    extract::Extension,
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

use crate::entities::users;
use crate::AppState;
use crate::errors::AppError;
use crate::request::{SyncUserRequest, BatchSyncRequest};
use crate::response::{SyncUserResponse, BatchSyncResponse};

/// POST /api/internal/sync-user
///
/// Syncs a user from juglans-api to jug0's users table.
/// This is an internal API called by juglans-api on user registration/update.
///
/// Authentication: Requires valid ORG-ID and ORG-KEY headers (handled by auth middleware)
pub async fn sync_user(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<SyncUserRequest>,
) -> Result<Json<SyncUserResponse>, AppError> {
    let org_id = req.org_id.unwrap_or_else(|| crate::official_org_slug().to_string());

    tracing::info!(
        "[User Sync] Syncing user: id={}, username={}, org={}",
        req.id, req.username, org_id
    );

    // Check if user already exists by external_id (juglans-api user ID)
    let existing_by_external = users::Entity::find()
        .filter(users::Column::ExternalId.eq(&req.id))
        .one(&state.db)
        .await?;

    // Also check by username to handle username changes
    let existing_by_username = users::Entity::find()
        .filter(users::Column::Username.eq(&req.username))
        .one(&state.db)
        .await?;

    let jug0_user_id: Uuid;
    let message: String;

    if let Some(existing) = existing_by_external {
        // User exists, update their info
        jug0_user_id = existing.id;

        let mut active: users::ActiveModel = existing.into();
        active.username = Set(Some(req.username.clone()));
        active.name = Set(req.name.clone());
        active.org_id = Set(Some(org_id));
        active.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
        active.update(&state.db).await?;

        message = format!("Updated existing user (external_id={})", req.id);
        tracing::info!("[User Sync] {}", message);

    } else if let Some(existing_username) = existing_by_username {
        // Username exists but with different external_id - update external_id
        // This handles the case where a user was created before sync was implemented
        jug0_user_id = existing_username.id;

        let mut active: users::ActiveModel = existing_username.into();
        active.external_id = Set(Some(req.id.clone()));
        active.name = Set(req.name.clone());
        active.org_id = Set(Some(org_id));
        active.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
        active.update(&state.db).await?;

        message = format!("Linked existing username '{}' to external_id={}", req.username, req.id);
        tracing::info!("[User Sync] {}", message);

    } else {
        // Create new user
        jug0_user_id = Uuid::new_v4();

        let new_user = users::ActiveModel {
            id: Set(jug0_user_id),
            external_id: Set(Some(req.id.clone())),
            org_id: Set(Some(org_id)),
            username: Set(Some(req.username.clone())),
            name: Set(req.name.clone()),
            role: Set("user".to_string()),
            created_at: Set(Some(chrono::Utc::now().naive_utc())),
            ..Default::default()
        };
        new_user.insert(&state.db).await?;

        message = format!("Created new user: username={}, external_id={}", req.username, req.id);
        tracing::info!("[User Sync] {}", message);
    }

    Ok(Json(SyncUserResponse {
        success: true,
        jug0_user_id: jug0_user_id.to_string(),
        message,
    }))
}

/// POST /api/internal/sync-users (batch)
///
/// Syncs multiple users at once. Used for initial migration.
pub async fn batch_sync_users(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<BatchSyncRequest>,
) -> Result<Json<BatchSyncResponse>, AppError> {
    tracing::info!("[User Sync] Batch syncing {} users", req.users.len());

    let mut synced = 0;
    let mut errors = Vec::new();

    for user_req in req.users {
        let username = user_req.username.clone();
        match sync_user_internal(&state, user_req).await {
            Ok(_) => synced += 1,
            Err(e) => {
                let err_msg = format!("Failed to sync user '{}': {}", username, e);
                tracing::warn!("[User Sync] {}", err_msg);
                errors.push(err_msg);
            }
        }
    }

    tracing::info!("[User Sync] Batch complete: {} synced, {} errors", synced, errors.len());

    Ok(Json(BatchSyncResponse {
        success: errors.is_empty(),
        synced,
        errors,
    }))
}

/// Internal sync logic (shared between single and batch)
async fn sync_user_internal(state: &Arc<AppState>, req: SyncUserRequest) -> Result<Uuid, AppError> {
    let org_id = req.org_id.unwrap_or_else(|| crate::official_org_slug().to_string());

    let existing = users::Entity::find()
        .filter(users::Column::ExternalId.eq(&req.id))
        .one(&state.db)
        .await?;

    if let Some(existing) = existing {
        let user_id = existing.id;
        let mut active: users::ActiveModel = existing.into();
        active.username = Set(Some(req.username));
        active.name = Set(req.name);
        active.org_id = Set(Some(org_id));
        active.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
        active.update(&state.db).await?;
        Ok(user_id)
    } else {
        let user_id = Uuid::new_v4();
        let new_user = users::ActiveModel {
            id: Set(user_id),
            external_id: Set(Some(req.id)),
            org_id: Set(Some(org_id)),
            username: Set(Some(req.username)),
            name: Set(req.name),
            role: Set("user".to_string()),
            created_at: Set(Some(chrono::Utc::now().naive_utc())),
            ..Default::default()
        };
        new_user.insert(&state.db).await?;
        Ok(user_id)
    }
}