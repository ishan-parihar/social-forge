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
    pub status: String,                  // active | paused | archived | completed
    pub progress_metric: Option<String>, // posts | engagement | reach | followers | custom
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
    /// Total engagement (likes + comments + shares) across this
    /// campaign's published posts, from `analytics_cache`.
    ///
    /// The progress bar measures `progress_metric`, which can be
    /// "engagement" - so it needs the engagement number, not the post
    /// count. `None` when the campaign has no measured posts: "not
    /// measured" is not "zero".
    ///
    /// Not a column: it is computed in a second query and set on the
    /// row afterwards, so `FromRow` must not look for it.
    #[sqlx(skip)]
    pub engagement_total: Option<i64>,
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
    /// A campaign always has a colour, so there is nothing to clear:
    /// two-state.
    pub color: Option<String>,
    /// The clearable fields are THREE-state:
    ///   absent  -> keep the stored value
    ///   `null`  -> clear it
    ///   a value -> replace it
    ///
    /// They used to be a plain `Option<T>` with `COALESCE(?, col)` in
    /// the SQL, and `COALESCE` cannot tell "keep" from "clear" - so
    /// emptying the Goal field and pressing Save reported "Campaign
    /// updated" and left the old goal in place. The same for the
    /// description, the start and end dates, the progress metric and
    /// the progress target. The settings page even offers a "None"
    /// option for the progress metric that could never be applied.
    ///
    /// `name`, `color` and `status` stay two-state: a campaign always
    /// has a name, a colour and a status, so there is nothing to clear.
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub end_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub goal: Option<Option<String>>,
    // v22 Phase 6: expanded fields on update.
    pub status: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub progress_metric: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub progress_target: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub audience_persona: Option<Option<serde_json::Value>>,
    #[serde(default, deserialize_with = "double_option")]
    pub content_pillars: Option<Option<serde_json::Value>>,
    #[serde(default, deserialize_with = "double_option")]
    pub budget_cents: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub kpi_targets: Option<Option<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStageRequest {
    pub state: String,
    /// Three states, not two:
    ///   absent  -> leave the campaign alone
    ///   `null`  -> unassign the post from its campaign
    ///   a value -> assign the post to that campaign
    ///
    /// This was a plain `Option<Uuid>`, and the UPDATE assigned
    /// `campaign_id = ?` unconditionally - so a caller that simply did not
    /// mention the campaign (the kanban drag, with no campaign filter
    /// active) wrote NULL and silently dropped the post out of its
    /// campaign. The frontend contract already said "omitted = keep the
    /// existing value" for `kanban_substate` and `priority`; `campaign_id`
    /// now behaves the same way.
    #[serde(default, deserialize_with = "double_option")]
    pub campaign_id: Option<Option<Uuid>>,
    // v22 Phase 6: kanban fields that can be updated alongside state.
    pub kanban_substate: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<String>,
}

/// Deserialize a field into `Option<Option<T>>` so "absent", "null" and
/// "a value" are three distinguishable states.
///
/// `Option<T>` collapses "absent" and "null" into the same `None`, which
/// is why a plain `Option<Uuid>` could not express "leave it alone" vs
/// "clear it".
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
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
                  COUNT(p.id) AS post_count
           FROM campaigns c
           LEFT JOIN posts p ON p.campaign_id = c.id AND p.deleted_at IS NULL
           WHERE c.user_id = ? AND c.deleted_at IS NULL
           GROUP BY c.id
           ORDER BY c.sort_order ASC, c.created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch campaigns: {e}")))?;

    // The engagement total for each campaign, from `analytics_cache`.
    //
    // `progress_metric` can be "posts", "engagement", "reach",
    // "followers" or "custom", but the list only carried `post_count` -
    // so a campaign with `progress_metric = "engagement"` and a target
    // of 1000 measured its POST count against it and showed "0 / 1000
    // engagement" with an empty bar. The metric the founder chose was
    // not the number being measured.
    //
    // Own-post engagement lives in `analytics_cache`, keyed by
    // (user_id, platform_post_id) - NOT in `post_engagement`, whose
    // `post_id` FK targets `external_posts`.
    let engagement_by_campaign: std::collections::HashMap<Uuid, i64> = {
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            r#"SELECT p.campaign_id,
                      COALESCE(SUM(json_extract(point.value, '$.total')), 0) AS engagement
               FROM posts p
               JOIN analytics_cache ac ON ac.user_id = p.user_id
                                     AND ac.platform_post_id = p.platform_post_id
               CROSS JOIN json_each(ac.data) AS series
               CROSS JOIN json_each(json_extract(series.value, '$.data')) AS point
               WHERE p.user_id = ?
                 AND p.campaign_id IS NOT NULL
                 AND p.deleted_at IS NULL
                 AND lower(json_extract(series.value, '$.label'))
                     IN ('likes', 'comments', 'shares', 'like', 'comment', 'share',
                         'retweets', 'reposts', 'replies', 'reply', 'repost')
               GROUP BY p.campaign_id"#,
        )
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        rows.into_iter().collect()
    };

    let campaigns: Vec<Campaign> = campaigns
        .into_iter()
        .map(|mut c| {
            c.engagement_total = engagement_by_campaign.get(&c.id).copied();
            c
        })
        .collect();

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
    let start_date = body
        .start_date
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = body
        .end_date
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let status = body.status.unwrap_or_else(|| "active".into());

    // Validate status.
    if !["active", "paused", "archived", "completed"].contains(&status.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid status: {status}")));
    }

    let campaign: Campaign = sqlx::query_as(
        r#"INSERT INTO campaigns (
               id, user_id, name, description, color, start_date, end_date, goal,
               status, progress_metric, progress_target, audience_persona,
               content_pillars, budget_cents, kpi_targets
           )
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING id, user_id, name, description, color, start_date, end_date, goal,
                     status, progress_metric, progress_target, audience_persona,
                     content_pillars, budget_cents, kpi_targets, sort_order,
                     deleted_at, created_at, updated_at, NULL AS post_count"#,
    )
    .bind(Uuid::new_v4())
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
    // A date is three-state too: absent keeps it, `null` clears it, a
    // value replaces it. An unparseable date is a 400, not a silent
    // "keep the old one" - the old code dropped the parse failure on
    // the floor, so a typo in the date field was accepted and ignored.
    let parse_date = |v: &Option<Option<String>>| -> Result<Option<Option<chrono::NaiveDate>>, AppError> {
        match v {
            None => Ok(None),
            Some(None) => Ok(Some(None)),
            Some(Some(s)) => match chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
                Ok(d) => Ok(Some(Some(d))),
                Err(_) => Err(AppError::BadRequest(format!(
                    "Invalid date '{s}' - use YYYY-MM-DD"
                ))),
            },
        }
    };
    let start_date = parse_date(&body.start_date)?;
    let end_date = parse_date(&body.end_date)?;

    // Validate status if provided.
    if let Some(status) = &body.status {
        if !["active", "paused", "archived", "completed"].contains(&status.as_str()) {
            return Err(AppError::BadRequest(format!("Invalid status: {status}")));
        }
    }

    // Each clearable field is written through a flag, because
    // `COALESCE(?, col)` cannot tell "keep" from "clear":
    //   flag = 0 -> leave the column alone
    //   flag = 1 -> write the value (which may be NULL, to clear)
    fn three_state<T: Clone>(v: &Option<Option<T>>) -> (i64, Option<T>) {
        match v {
            None => (0, None),
            Some(None) => (1, None),
            Some(Some(x)) => (1, Some(x.clone())),
        }
    }
    let (description_set, description) = three_state(&body.description);
    let (start_set, start_value) = three_state(&start_date);
    let (end_set, end_value) = three_state(&end_date);
    let (goal_set, goal) = three_state(&body.goal);
    let (metric_set, metric) = three_state(&body.progress_metric);
    let (target_set, target) = three_state(&body.progress_target);
    let (persona_set, persona) = three_state(&body.audience_persona);
    let (pillars_set, pillars) = three_state(&body.content_pillars);
    let (budget_set, budget) = three_state(&body.budget_cents);
    let (kpi_set, kpi) = three_state(&body.kpi_targets);

    // SQLite has no `$N` positional params — `?` is positional in textual
    // order, so the SET placeholders come before the WHERE ones.
    let campaign: Campaign = sqlx::query_as(
        r#"UPDATE campaigns SET
             name = COALESCE(?, name),
             color = COALESCE(?, color),
             status = COALESCE(?, status),
             description = CASE WHEN ? THEN ? ELSE description END,
             start_date = CASE WHEN ? THEN ? ELSE start_date END,
             end_date = CASE WHEN ? THEN ? ELSE end_date END,
             goal = CASE WHEN ? THEN ? ELSE goal END,
             progress_metric = CASE WHEN ? THEN ? ELSE progress_metric END,
             progress_target = CASE WHEN ? THEN ? ELSE progress_target END,
             audience_persona = CASE WHEN ? THEN ? ELSE audience_persona END,
             content_pillars = CASE WHEN ? THEN ? ELSE content_pillars END,
             budget_cents = CASE WHEN ? THEN ? ELSE budget_cents END,
             kpi_targets = CASE WHEN ? THEN ? ELSE kpi_targets END,
             updated_at = unixepoch()
           WHERE id = ? AND user_id = ? AND deleted_at IS NULL
           RETURNING id, user_id, name, description, color, start_date, end_date, goal,
                     status, progress_metric, progress_target, audience_persona,
                     content_pillars, budget_cents, kpi_targets, sort_order,
                     deleted_at, created_at, updated_at, NULL AS post_count"#,
    )
    .bind(&body.name)
    .bind(&body.color)
    .bind(&body.status)
    .bind(description_set)
    .bind(description)
    .bind(start_set)
    .bind(start_value)
    .bind(end_set)
    .bind(end_value)
    .bind(goal_set)
    .bind(goal)
    .bind(metric_set)
    .bind(metric)
    .bind(target_set)
    .bind(target)
    .bind(persona_set)
    .bind(persona)
    .bind(pillars_set)
    .bind(pillars)
    .bind(budget_set)
    .bind(budget)
    .bind(kpi_set)
    .bind(kpi)
    .bind(id)
    .bind(auth.user_id)
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
    let affected = sqlx::query("UPDATE campaigns SET deleted_at = unixepoch(), status = 'archived', updated_at = unixepoch() WHERE id = ? AND user_id = ? AND deleted_at IS NULL")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete campaign: {e}")))?
        .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound("Campaign not found".into()));
    }

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
        return Err(AppError::BadRequest(format!(
            "Invalid state: {}",
            body.state
        )));
    }

    // v25-3: validate kanban metadata fields if provided. The DB has CHECK
    // constraints (migration 034) but we want a friendly error before the
    // UPDATE rather than a raw CHECK violation.
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
    // Parse due_date if provided (RFC3339 → epoch i64; the column is an
    // INTEGER epoch). An empty string is treated as "clear" (set to NULL).
    let due_date_dt: Option<Option<i64>> = match &body.due_date {
        None => None,                                 // don't touch the field
        Some(s) if s.trim().is_empty() => Some(None), // explicit clear
        Some(s) => {
            let dt = chrono::DateTime::parse_from_rfc3339(s).map_err(|_| {
                AppError::BadRequest("Invalid due_date format, use ISO8601/RFC3339".into())
            })?;
            Some(Some(dt.timestamp()))
        }
    };

    // `campaign_id` is three-state. `COALESCE(?, campaign_id)` cannot
    // express "clear", because a NULL bind is indistinguishable from an
    // absent one - so the assignment is gated on an explicit flag:
    //   flag = 0 -> leave the campaign alone
    //   flag = 1 -> write the value (which may be NULL, to unassign)
    let (campaign_set, campaign_value): (i64, Option<Uuid>) = match &body.campaign_id {
        None => (0, None),
        Some(None) => (1, None),
        Some(Some(c)) => (1, Some(*c)),
    };

    // Fetch the current state to validate the transition.
    let current_state: Option<String> =
        sqlx::query_scalar(r#"SELECT state FROM posts WHERE id = ? AND user_id = ?"#)
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
        if let Some(due_date_dt) = due_date_dt {
            // Explicit set or clear.
            sqlx::query(
                r#"UPDATE posts SET
                      campaign_id = CASE WHEN ? THEN ? ELSE campaign_id END,
                      kanban_substate = COALESCE(?, kanban_substate),
                      priority = COALESCE(?, priority),
                      due_date = ?,
                      updated_at = unixepoch()
                    WHERE id = ? AND user_id = ?"#,
            )
            .bind(campaign_set)
            .bind(campaign_value)
            .bind(&body.kanban_substate)
            .bind(&body.priority)
            .bind(due_date_dt)
            .bind(id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update post: {e}")))?;
        } else {
            // Leave due_date unchanged.
            sqlx::query(
                r#"UPDATE posts SET
                     campaign_id = CASE WHEN ? THEN ? ELSE campaign_id END,
                     kanban_substate = COALESCE(?, kanban_substate),
                     priority = COALESCE(?, priority),
                     updated_at = unixepoch()
                   WHERE id = ? AND user_id = ?"#,
            )
            .bind(campaign_set)
            .bind(campaign_value)
            .bind(&body.kanban_substate)
            .bind(&body.priority)
            .bind(id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update post: {e}")))?;
        }

        // The post's campaign AFTER the write, so the broadcast reports the
        // effective value (unchanged / assigned / unassigned).
        let effective_campaign_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT campaign_id FROM posts WHERE id = ? AND user_id = ?",
        )
        .bind(id)
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        state.broadcast.send(
            "post_stage_changed",
            &serde_json::json!({
                "id": id.to_string(),
                "state": body.state,
                "previous_state": current_state,
                "campaign_id": effective_campaign_id.map(|c| c.to_string()),
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
    if let Some(due_date_dt) = due_date_dt {
        sqlx::query(
            r#"UPDATE posts SET
                  state = ?,
                  campaign_id = CASE WHEN ? THEN ? ELSE campaign_id END,
                  kanban_substate = COALESCE(?, kanban_substate),
                  priority = COALESCE(?, priority),
                  due_date = ?,
                  updated_at = unixepoch()
                WHERE id = ? AND user_id = ?"#,
        )
        .bind(&body.state)
        .bind(campaign_set)
        .bind(campaign_value)
        .bind(&body.kanban_substate)
        .bind(&body.priority)
        .bind(due_date_dt)
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update post stage: {e}")))?;
    } else {
        sqlx::query(
            r#"UPDATE posts SET
                 state = ?,
                 campaign_id = CASE WHEN ? THEN ? ELSE campaign_id END,
                 kanban_substate = COALESCE(?, kanban_substate),
                 priority = COALESCE(?, priority),
                 updated_at = unixepoch()
               WHERE id = ? AND user_id = ?"#,
        )
        .bind(&body.state)
        .bind(campaign_set)
        .bind(campaign_value)
        .bind(&body.kanban_substate)
        .bind(&body.priority)
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update post stage: {e}")))?;
    }

    // Broadcast so other tabs / the dashboard update in real-time.
    // The broadcast carries the post's campaign AFTER the write, so a
    // listener sees the effective value - including "unchanged" (the
    // post kept its campaign) and "unassigned" (it lost it).
    let effective_campaign_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT campaign_id FROM posts WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    state.broadcast.send(
        "post_stage_changed",
        &serde_json::json!({
            "id": id.to_string(),
            "state": body.state,
            "previous_state": current_state,
            "campaign_id": effective_campaign_id.map(|c| c.to_string()),
        }),
    );

    Ok(Json(serde_json::json!({ "updated": true })))
}
