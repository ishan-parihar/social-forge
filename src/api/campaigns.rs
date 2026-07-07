// ─── Campaigns API Routes ──────────────────────────────────────
// CRUD for campaign entities + post stage management (kanban).
//
// A campaign is a named group of posts with a color, description, and
// optional date range. The kanban board groups posts by post_state
// (idea, draft, queued, published) and optionally by campaign.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::PgPool;
use crate::error::AppError;

use super::AppState;

// ── Types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Campaign {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub goal: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Number of posts in this campaign (joined).
    pub post_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStageRequest {
    pub state: String,
    pub campaign_id: Option<Uuid>,
}

// ── Handlers ──────────────────────────────────────────────────

/// GET /api/campaigns — list all campaigns for the user
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<Campaign>>, AppError> {
    let campaigns: Vec<Campaign> = sqlx::query_as(
        r#"SELECT c.*, COUNT(p.id)::bigint AS post_count
           FROM campaigns c
           LEFT JOIN posts p ON p.campaign_id = c.id
           WHERE c.user_id = $1
           GROUP BY c.id
           ORDER BY c.created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch campaigns: {e}")))?;

    Ok(Json(campaigns))
}

/// POST /api/campaigns — create a new campaign
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateCampaignRequest>,
) -> Result<Json<Campaign>, AppError> {
    let color = body.color.unwrap_or_else(|| "#6366f1".into());
    let start_date = body.start_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = body.end_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let campaign: Campaign = sqlx::query_as(
        r#"INSERT INTO campaigns (user_id, name, description, color, start_date, end_date, goal)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, user_id, name, description, color, start_date, end_date, goal,
                     created_at, updated_at, NULL::bigint AS post_count"#,
    )
    .bind(auth.user_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&color)
    .bind(start_date)
    .bind(end_date)
    .bind(&body.goal)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create campaign: {e}")))?;

    Ok(Json(campaign))
}

/// PUT /api/campaigns/{id} — update a campaign
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCampaignRequest>,
) -> Result<Json<Campaign>, AppError> {
    let start_date = body.start_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = body.end_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let campaign: Campaign = sqlx::query_as(
        r#"UPDATE campaigns SET
             name = COALESCE($3, name),
             description = COALESCE($4, description),
             color = COALESCE($5, color),
             start_date = COALESCE($6, start_date),
             end_date = COALESCE($7, end_date),
             goal = COALESCE($8, goal),
             updated_at = NOW()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, name, description, color, start_date, end_date, goal,
                     created_at, updated_at, NULL::bigint AS post_count"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.color)
    .bind(start_date)
    .bind(end_date)
    .bind(&body.goal)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update campaign: {e}")))?;

    Ok(Json(campaign))
}

/// DELETE /api/campaigns/{id} — delete a campaign (posts keep their campaign_id as NULL)
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM campaigns WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete campaign: {e}")))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// PATCH /api/posts/{id}/stage — change a post's state (kanban drag-and-drop)
/// and optionally assign it to a campaign.
///
/// v22 Phase 1 (BUG #6): State-transition validation. Previously a user
/// could drag a post from `idea` → `published` even if it had no
/// `platform_post_id` — the DB happily wrote `state='published'` with
/// `published_at IS NULL` and the calendar then treated it as published
/// (filtering by `published_at`), making it disappear. Now we reject
/// illegal transitions and require `platform_post_id` for `published`.
///
/// v22 Phase 1 (BUG #7): Broadcast `post_stage_changed` so other browser
/// tabs (and the dashboard) update in real-time. Previously the kanban
/// had no multi-tab sync — dragging in tab A left tab B stale.
pub async fn update_stage(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateStageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Validate the state value.
    let valid_states = ["idea", "draft", "queued", "published", "error"];
    if !valid_states.contains(&body.state.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid state: {}", body.state)));
    }

    // Fetch the current state to validate the transition.
    let current_state: Option<String> = sqlx::query_scalar(
        r#"SELECT state::text FROM posts WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch current state: {e}")))?;

    let current_state = current_state.ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    // No-op if the state isn't actually changing (but still update campaign_id).
    if current_state == body.state {
        sqlx::query(
            r#"UPDATE posts SET campaign_id = $3, updated_at = NOW()
               WHERE id = $1 AND user_id = $2"#,
        )
        .bind(id)
        .bind(auth.user_id)
        .bind(body.campaign_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update campaign: {e}")))?;

        state.broadcast.send(
            "post_stage_changed",
            &serde_json::json!({
                "id": id.to_string(),
                "state": body.state,
                "previous_state": current_state,
                "campaign_id": body.campaign_id.map(|c| c.to_string()),
            }),
        );
        return Ok(Json(serde_json::json!({ "updated": true, "noop": true })));
    }

    // Validate the transition is legal.
    // Allowed forward transitions:
    //   idea → draft, idea → error
    //   draft → queued, draft → error, draft → idea
    //   queued → publishing (scheduler only — not user-settable here),
    //           queued → error, queued → draft, queued → idea
    //   published → queued (reschedule), published → error, published → draft
    //   error → draft, error → queued, error → idea
    // The scheduler is the only thing that should set `publishing` and
    // `published` (with platform_post_id); users can move posts OUT of
    // `published` (e.g. back to draft to edit) but not INTO it manually.
    let allowed = match (&current_state[..], body.state.as_str()) {
        // Forward out of idea
        ("idea", "draft") | ("idea", "error") => true,
        // Forward out of draft
        ("draft", "queued") | ("draft", "error") | ("draft", "idea") => true,
        // Forward out of queued (NOT to publishing — that's scheduler-only)
        ("queued", "error") | ("queued", "draft") | ("queued", "idea") => true,
        // Forward out of published (reschedule / re-draft)
        ("published", "queued") | ("published", "error") | ("published", "draft") => true,
        // Forward out of error
        ("error", "draft") | ("error", "queued") | ("error", "idea") => true,
        // `published` is only reachable via the scheduler — reject manual
        // attempts to drag a post into the Published column.
        (_, "published") => false,
        // `publishing` is scheduler-only — never user-settable.
        (_, "publishing") => false,
        _ => false,
    };
    if !allowed {
        return Err(AppError::BadRequest(format!(
            "Illegal state transition: {current_state} → {} (only the scheduler can publish posts)",
            body.state
        )));
    }

    sqlx::query(
        r#"UPDATE posts SET
             state = $3::post_state,
             campaign_id = $4,
             updated_at = NOW()
           WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&body.state)
    .bind(body.campaign_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update post stage: {e}")))?;

    // Broadcast so other tabs / the dashboard update in real-time.
    state.broadcast.send(
        "post_stage_changed",
        &serde_json::json!({
            "id": id.to_string(),
            "state": body.state,
            "previous_state": current_state,
            "campaign_id": body.campaign_id.map(|c| c.to_string()),
        }),
    );

    Ok(Json(serde_json::json!({ "updated": true })))
}
