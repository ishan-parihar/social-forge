// ─── Campaigns API Routes ──────────────────────────────────────
// CRUD for campaign entities + post stage management (kanban).
//
// A campaign is a named group of posts with a color, description, and
// optional date range. The kanban board groups posts by post_state
// (idea, draft, queued, published) and optionally by campaign.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
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
    // v22 Phase 6: expanded campaign fields.
    pub status: String,                    // active | paused | archived | completed
    pub progress_metric: Option<String>,   // posts | engagement | reach | followers | custom
    pub progress_target: Option<i32>,
    pub audience_persona: Option<serde_json::Value>,
    pub content_pillars: Option<serde_json::Value>,
    pub budget_cents: Option<i32>,
    pub kpi_targets: Option<serde_json::Value>,
    pub sort_order: i32,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
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
    // v22 Phase 6: optional expanded fields on create.
    pub status: Option<String>,
    pub progress_metric: Option<String>,
    pub progress_target: Option<i32>,
    pub audience_persona: Option<serde_json::Value>,
    pub content_pillars: Option<serde_json::Value>,
    pub budget_cents: Option<i32>,
    pub kpi_targets: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub goal: Option<String>,
    // v22 Phase 6: expanded fields on update.
    pub status: Option<String>,
    pub progress_metric: Option<String>,
    pub progress_target: Option<i32>,
    pub audience_persona: Option<serde_json::Value>,
    pub content_pillars: Option<serde_json::Value>,
    pub budget_cents: Option<i32>,
    pub kpi_targets: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStageRequest {
    pub state: String,
    pub campaign_id: Option<Uuid>,
    // v22 Phase 6: kanban fields that can be updated alongside state.
    pub kanban_substate: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────

/// GET /api/campaigns — list all campaigns for the user.
/// v22 Phase 6: filters out soft-deleted campaigns. Archived campaigns
/// are included by default so the user can see them in the UI; the
/// frontend can filter by status if needed.
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<Campaign>>, AppError> {
    let campaigns: Vec<Campaign> = sqlx::query_as(
        r#"SELECT c.id, c.user_id, c.name, c.description, c.color,
                  c.start_date, c.end_date, c.goal, c.status,
                  c.progress_metric, c.progress_target, c.audience_persona,
                  c.content_pillars, c.budget_cents, c.kpi_targets,
                  c.sort_order, c.deleted_at, c.created_at, c.updated_at,
                  COUNT(p.id)::bigint AS post_count
           FROM campaigns c
           LEFT JOIN posts p ON p.campaign_id = c.id AND p.deleted_at IS NULL
           WHERE c.user_id = $1 AND c.deleted_at IS NULL
           GROUP BY c.id
           ORDER BY c.sort_order ASC, c.created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch campaigns: {e}")))?;

    Ok(Json(campaigns))
}

/// POST /api/campaigns — create a new campaign.
/// v22 Phase 6: accepts the expanded fields (status, progress_metric,
/// progress_target, audience_persona, content_pillars, budget_cents,
/// kpi_targets).
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateCampaignRequest>,
) -> Result<Json<Campaign>, AppError> {
    let color = body.color.unwrap_or_else(|| "#6366f1".into());
    let start_date = body.start_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = body.end_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let status = body.status.unwrap_or_else(|| "active".into());

    // Validate status.
    if !["active", "paused", "archived", "completed"].contains(&status.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid status: {status}")));
    }

    let campaign: Campaign = sqlx::query_as(
        r#"INSERT INTO campaigns (
               user_id, name, description, color, start_date, end_date, goal,
               status, progress_metric, progress_target, audience_persona,
               content_pillars, budget_cents, kpi_targets
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
           RETURNING id, user_id, name, description, color, start_date, end_date, goal,
                     status, progress_metric, progress_target, audience_persona,
                     content_pillars, budget_cents, kpi_targets, sort_order,
                     deleted_at, created_at, updated_at, NULL::bigint AS post_count"#,
    )
    .bind(auth.user_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&color)
    .bind(start_date)
    .bind(end_date)
    .bind(&body.goal)
    .bind(&status)
    .bind(&body.progress_metric)
    .bind(body.progress_target)
    .bind(&body.audience_persona)
    .bind(&body.content_pillars)
    .bind(body.budget_cents)
    .bind(&body.kpi_targets)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create campaign: {e}")))?;

    // v22 Phase 6: broadcast campaign_created so other tabs update.
    state.broadcast.send(
        "campaign_created",
        &serde_json::json!({"id": campaign.id.to_string(), "name": campaign.name}),
    );

    Ok(Json(campaign))
}

/// PUT /api/campaigns/{id} — update a campaign.
/// v22 Phase 6: accepts all expanded fields. The frontend's
/// campaignsApi.update() was previously dead code — now it's wired up.
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCampaignRequest>,
) -> Result<Json<Campaign>, AppError> {
    let start_date = body.start_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = body.end_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    // Validate status if provided.
    if let Some(ref status) = body.status {
        if !["active", "paused", "archived", "completed"].contains(&status.as_str()) {
            return Err(AppError::BadRequest(format!("Invalid status: {status}")));
        }
    }

    let campaign: Campaign = sqlx::query_as(
        r#"UPDATE campaigns SET
             name = COALESCE($3, name),
             description = COALESCE($4, description),
             color = COALESCE($5, color),
             start_date = COALESCE($6, start_date),
             end_date = COALESCE($7, end_date),
             goal = COALESCE($8, goal),
             status = COALESCE($9, status),
             progress_metric = COALESCE($10, progress_metric),
             progress_target = COALESCE($11, progress_target),
             audience_persona = COALESCE($12, audience_persona),
             content_pillars = COALESCE($13, content_pillars),
             budget_cents = COALESCE($14, budget_cents),
             kpi_targets = COALESCE($15, kpi_targets),
             updated_at = NOW()
           WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
           RETURNING id, user_id, name, description, color, start_date, end_date, goal,
                     status, progress_metric, progress_target, audience_persona,
                     content_pillars, budget_cents, kpi_targets, sort_order,
                     deleted_at, created_at, updated_at, NULL::bigint AS post_count"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.color)
    .bind(start_date)
    .bind(end_date)
    .bind(&body.goal)
    .bind(&body.status)
    .bind(&body.progress_metric)
    .bind(body.progress_target)
    .bind(&body.audience_persona)
    .bind(&body.content_pillars)
    .bind(body.budget_cents)
    .bind(&body.kpi_targets)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update campaign: {e}")))?;

    // v22 Phase 6: broadcast campaign_updated.
    state.broadcast.send(
        "campaign_updated",
        &serde_json::json!({"id": campaign.id.to_string(), "name": campaign.name}),
    );

    Ok(Json(campaign))
}

/// DELETE /api/campaigns/{id} — soft-delete a campaign (v22 Phase 6, BUG #10).
/// Previously this was a hard delete with no recovery. Now sets deleted_at
/// so the campaign can be restored. Posts keep their campaign_id (the FK
/// has ON DELETE SET NULL, but we no longer DELETE — we UPDATE).
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE campaigns SET deleted_at = NOW(), status = 'archived', updated_at = NOW() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete campaign: {e}")))?;

    // v22 Phase 6: broadcast campaign_deleted.
    state.broadcast.send(
        "campaign_deleted",
        &serde_json::json!({"id": id.to_string()}),
    );

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

    // v25-3: validate kanban metadata fields if provided. The DB has CHECK
    // constraints (migration 034) but we want a friendly error before the
    // UPDATE rather than a raw Postgres violation.
    if let Some(ref sub) = body.kanban_substate {
        if !["ready_to_publish", "in_review", "blocked"].contains(&sub.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid kanban_substate: {sub} (expected ready_to_publish | in_review | blocked)"
            )));
        }
    }
    if let Some(ref pri) = body.priority {
        if !["low", "medium", "high", "urgent"].contains(&pri.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid priority: {pri} (expected low | medium | high | urgent)"
            )));
        }
    }
    // Parse due_date if provided (RFC3339 → DateTime<Utc>). An empty string
    // is treated as "clear" (set to NULL).
    let due_date_dt: Option<Option<chrono::DateTime<chrono::Utc>>> = match &body.due_date {
        None => None, // don't touch the field
        Some(s) if s.trim().is_empty() => Some(None), // explicit clear
        Some(s) => {
            let dt = chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|_| AppError::BadRequest("Invalid due_date format, use ISO8601/RFC3339".into()))?
                .with_timezone(&chrono::Utc);
            Some(Some(dt))
        }
    };

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

    // No-op if the state isn't actually changing (but still update campaign_id
    // + any kanban metadata fields the caller passed).
    if current_state == body.state {
        // v25-3: persist kanban_substate / priority / due_date alongside
        // campaign_id. COALESCE keeps the existing value when the field
        // is None (caller didn't pass it). For due_date, we use a separate
        // branch because "clear" (Some(None)) vs "unchanged" (None) vs
        // "set" (Some(Some(dt))) are three distinct states.
        if due_date_dt.is_some() {
            // Explicit set or clear.
            sqlx::query(
                r#"UPDATE posts SET
                     campaign_id = $3,
                     kanban_substate = COALESCE($4, kanban_substate),
                     priority = COALESCE($5, priority),
                     due_date = $6,
                     updated_at = NOW()
                   WHERE id = $1 AND user_id = $2"#,
            )
            .bind(id)
            .bind(auth.user_id)
            .bind(body.campaign_id)
            .bind(&body.kanban_substate)
            .bind(&body.priority)
            .bind(due_date_dt.unwrap())
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update post: {e}")))?;
        } else {
            // Leave due_date unchanged.
            sqlx::query(
                r#"UPDATE posts SET
                     campaign_id = $3,
                     kanban_substate = COALESCE($4, kanban_substate),
                     priority = COALESCE($5, priority),
                     updated_at = NOW()
                   WHERE id = $1 AND user_id = $2"#,
            )
            .bind(id)
            .bind(auth.user_id)
            .bind(body.campaign_id)
            .bind(&body.kanban_substate)
            .bind(&body.priority)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update post: {e}")))?;
        }

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

    // v25-3: same kanban-metadata persistence as the no-op branch above,
    // but the UPDATE also transitions state. Two branches for the same
    // due_date three-state reason (set / clear / unchanged).
    if due_date_dt.is_some() {
        sqlx::query(
            r#"UPDATE posts SET
                 state = $3::post_state,
                 campaign_id = $4,
                 kanban_substate = COALESCE($5, kanban_substate),
                 priority = COALESCE($6, priority),
                 due_date = $7,
                 updated_at = NOW()
               WHERE id = $1 AND user_id = $2"#,
        )
        .bind(id)
        .bind(auth.user_id)
        .bind(&body.state)
        .bind(body.campaign_id)
        .bind(&body.kanban_substate)
        .bind(&body.priority)
        .bind(due_date_dt.unwrap())
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update post stage: {e}")))?;
    } else {
        sqlx::query(
            r#"UPDATE posts SET
                 state = $3::post_state,
                 campaign_id = $4,
                 kanban_substate = COALESCE($5, kanban_substate),
                 priority = COALESCE($6, priority),
                 updated_at = NOW()
               WHERE id = $1 AND user_id = $2"#,
        )
        .bind(id)
        .bind(auth.user_id)
        .bind(&body.state)
        .bind(body.campaign_id)
        .bind(&body.kanban_substate)
        .bind(&body.priority)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update post stage: {e}")))?;
    }

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
