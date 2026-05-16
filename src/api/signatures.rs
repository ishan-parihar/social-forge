use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::middleware::AuthenticatedUser;
use crate::db::queries;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct CreateSignatureRequest {
    pub name: String,
    pub content: String,
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSignatureRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SignatureResponse {
    pub id: Uuid,
    pub name: String,
    pub content: String,
    pub provider: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::db::models::Signature> for SignatureResponse {
    fn from(s: crate::db::models::Signature) -> Self {
        Self {
            id: s.id,
            name: s.name,
            content: s.content,
            provider: s.provider,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

/// GET /api/signatures
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<SignatureResponse>>, AppError> {
    let signatures = queries::list_signatures(&state.db, auth.user_id).await?;
    Ok(Json(signatures.into_iter().map(SignatureResponse::from).collect()))
}

/// POST /api/signatures
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateSignatureRequest>,
) -> Result<Json<SignatureResponse>, AppError> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Signature name cannot be empty".into()));
    }
    let content = input.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::BadRequest("Signature content cannot be empty".into()));
    }

    let sig = queries::create_signature(
        &state.db,
        auth.user_id,
        &name,
        &content,
        input.provider.as_deref(),
    )
    .await?;

    Ok(Json(SignatureResponse::from(sig)))
}

/// PUT /api/signatures/{id}
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateSignatureRequest>,
) -> Result<Json<SignatureResponse>, AppError> {
    if let Some(ref name) = input.name {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest("Signature name cannot be empty".into()));
        }
    }
    if let Some(ref content) = input.content {
        if content.trim().is_empty() {
            return Err(AppError::BadRequest("Signature content cannot be empty".into()));
        }
    }
    let provider = input.provider.filter(|p| !p.trim().is_empty());

    let sig = queries::update_signature(
        &state.db,
        id,
        auth.user_id,
        input.name.as_deref(),
        input.content.as_deref(),
        provider.as_deref(),
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Signature not found".into()))?;

    Ok(Json(SignatureResponse::from(sig)))
}

/// DELETE /api/signatures/{id}
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = queries::delete_signature(&state.db, id, auth.user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Signature not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}
