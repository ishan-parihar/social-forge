// ─── Automation API Routes ──────────────────────────────────
// CRUD for automation rules and execution logs.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::types::EpochUtc;
use crate::error::AppError;

use super::AppState;

/// `id` is TEXT and `created_at` an INTEGER epoch. sqlx cannot decode `Uuid`
/// from TEXT (it wants 16 raw blob bytes, SQLite stores 36), so both come
/// back in their storage types and are rendered here.
#[derive(Debug, sqlx::FromRow)]
struct RuleRow {
    id: String,
    name: String,
    trigger_type: String,
    response_type: String,
    is_active: Option<i64>,
    created_at: Option<EpochUtc>,
}

// ── Request Types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListRulesQuery {
    pub integration_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub integration_id: Uuid,
    pub name: String,
    pub trigger_type: String,
    #[serde(default)]
    pub trigger_filter: serde_json::Value,
    pub response_template: String,
    pub response_type: String,
    pub ai_model: Option<String>,
    pub cooldown_minutes: Option<i32>,
    pub max_responses_per_hour: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub trigger_filter: Option<serde_json::Value>,
    pub response_template: Option<String>,
    pub response_type: Option<String>,
    pub ai_model: Option<String>,
    pub is_active: Option<bool>,
    pub cooldown_minutes: Option<i32>,
    pub max_responses_per_hour: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetLogsQuery {
    #[serde(default = "default_log_limit")]
    pub limit: i64,
}

fn default_log_limit() -> i64 {
    50
}

// ── Response Types ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RuleResponse {
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    pub response_type: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListRulesResponse {
    pub rules: Vec<RuleResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct CreateRuleResponse {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct LogEntryResponse {
    pub id: String,
    pub trigger_id: String,
    pub trigger_type: String,
    pub response: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct GetLogsResponse {
    pub logs: Vec<LogEntryResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

// ── Handlers ────────────────────────────────────────────────

/// GET /api/automation/rules?integration_id=X
pub async fn list_rules(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<ListRulesQuery>,
) -> Result<Json<ListRulesResponse>, AppError> {
    let rules: Vec<RuleRow> = if let Some(integration_id) = query.integration_id {
        sqlx::query_as!(
            RuleRow,
            r#"SELECT id as "id!: String", name, trigger_type, response_type,
                      is_active as "is_active: i64",
                      created_at as "created_at: EpochUtc"
               FROM automation_rules
               WHERE user_id = ? AND integration_id = ?
               ORDER BY created_at DESC"#,
            auth.user_id,
            integration_id,
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as!(
            RuleRow,
            r#"SELECT id as "id!: String", name, trigger_type, response_type,
                      is_active as "is_active: i64",
                      created_at as "created_at: EpochUtc"
               FROM automation_rules
               WHERE user_id = ?
               ORDER BY created_at DESC"#,
            auth.user_id,
        )
        .fetch_all(&state.db)
        .await?
    };

    let rule_responses: Vec<RuleResponse> = rules
        .into_iter()
        .map(|r| RuleResponse {
            id: r.id,
            name: r.name,
            trigger_type: r.trigger_type,
            response_type: r.response_type,
            is_active: r.is_active.map_or(true, |v| v != 0),
            created_at: r.created_at.unwrap_or_else(EpochUtc::now).0.to_rfc3339(),
        })
        .collect();

    let total = rule_responses.len();

    Ok(Json(ListRulesResponse {
        rules: rule_responses,
        total,
    }))
}

/// POST /api/automation/rules
pub async fn create_rule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateRuleRequest>,
) -> Result<Json<CreateRuleResponse>, AppError> {
    let cooldown = request.cooldown_minutes.unwrap_or(0);
    let max_per_hour = request.max_responses_per_hour.unwrap_or(10);

    let rule = sqlx::query!(
        r#"INSERT INTO automation_rules
           (user_id, integration_id, name, trigger_type, trigger_filter,
            response_template, response_type, ai_model, cooldown_minutes, max_responses_per_hour)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING id as "id!: String", name, is_active as "is_active: i64""#,
        auth.user_id,
        request.integration_id,
        request.name,
        request.trigger_type,
        request.trigger_filter,
        request.response_template,
        request.response_type,
        request.ai_model,
        cooldown,
        max_per_hour,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create rule: {e}")))?;

    Ok(Json(CreateRuleResponse {
        id: rule.id,
        name: rule.name,
        is_active: rule.is_active.map_or(true, |v| v != 0),
    }))
}

/// PUT /api/automation/rules/{id}
pub async fn update_rule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(rule_id): Path<Uuid>,
    Json(request): Json<UpdateRuleRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    // Verify ownership
    let existing = sqlx::query!(
        r#"SELECT id as "id!: String" FROM automation_rules WHERE id = ? AND user_id = ?"#,
        rule_id,
        auth.user_id,
    )
    .fetch_optional(&state.db)
    .await?;

    if existing.is_none() {
        return Err(AppError::NotFound("Rule not found".into()));
    }

    let request_active_i64 = request.is_active.map(i64::from);
    sqlx::query!(
        r#"UPDATE automation_rules
           SET name = COALESCE(?, name),
               trigger_filter = COALESCE(?, trigger_filter),
               response_template = COALESCE(?, response_template),
               response_type = COALESCE(?, response_type),
               ai_model = COALESCE(?, ai_model),
               is_active = COALESCE(?, is_active),
               cooldown_minutes = COALESCE(?, cooldown_minutes),
               max_responses_per_hour = COALESCE(?, max_responses_per_hour),
               updated_at = unixepoch()
           WHERE id = ? AND user_id = ?"#,
        rule_id,
        auth.user_id,
        request.name,
        request.trigger_filter,
        request.response_template,
        request.response_type,
        request.ai_model,
        request_active_i64,
        request.cooldown_minutes,
        request.max_responses_per_hour,
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update rule: {e}")))?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Rule updated".into(),
    }))
}

/// DELETE /api/automation/rules/{id}
pub async fn delete_rule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(rule_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, AppError> {
    let result = sqlx::query!(
        r#"DELETE FROM automation_rules WHERE id = ? AND user_id = ?"#,
        rule_id,
        auth.user_id,
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to delete rule: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Rule not found".into()));
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Rule deleted".into(),
    }))
}

/// GET /api/automation/rules/{id}/logs?limit=50
pub async fn get_logs(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(rule_id): Path<Uuid>,
    Query(query): Query<GetLogsQuery>,
) -> Result<Json<GetLogsResponse>, AppError> {
    // Verify ownership
    let existing = sqlx::query!(
        r#"SELECT id as "id!: String" FROM automation_rules WHERE id = ? AND user_id = ?"#,
        rule_id,
        auth.user_id,
    )
    .fetch_optional(&state.db)
    .await?;

    if existing.is_none() {
        return Err(AppError::NotFound("Rule not found".into()));
    }

    let logs = sqlx::query!(
        r#"SELECT id as "id!: String", trigger_id, trigger_type, response, status, error_message,
                  created_at as "created_at: EpochUtc"
           FROM automation_logs
           WHERE rule_id = ?
           ORDER BY created_at DESC
           LIMIT ?"#,
        rule_id,
        query.limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to get logs: {e}")))?;

    let log_responses: Vec<LogEntryResponse> = logs
        .into_iter()
        .map(|l| LogEntryResponse {
            id: l.id,
            trigger_id: l.trigger_id,
            trigger_type: l.trigger_type,
            response: l.response,
            status: l.status,
            error_message: l.error_message,
            created_at: l.created_at.unwrap_or_else(EpochUtc::now).0.to_rfc3339(),
        })
        .collect();

    let total = log_responses.len();

    Ok(Json(GetLogsResponse {
        logs: log_responses,
        total,
    }))
}
