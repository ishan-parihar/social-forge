// ─── Automation API Routes ──────────────────────────────────
// CRUD for automation rules and execution logs.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    name: String,
    trigger_type: String,
    response_type: String,
    is_active: Option<bool>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
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
            r#"SELECT id, name, trigger_type, response_type, is_active, created_at
               FROM automation_rules
               WHERE user_id = $1 AND integration_id = $2
               ORDER BY created_at DESC"#,
            auth.user_id,
            integration_id,
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as!(
            RuleRow,
            r#"SELECT id, name, trigger_type, response_type, is_active, created_at
               FROM automation_rules
               WHERE user_id = $1
               ORDER BY created_at DESC"#,
            auth.user_id,
        )
        .fetch_all(&state.db)
        .await?
    };

    let rule_responses: Vec<RuleResponse> = rules
        .into_iter()
        .map(|r| RuleResponse {
            id: r.id.to_string(),
            name: r.name,
            trigger_type: r.trigger_type,
            response_type: r.response_type,
            is_active: r.is_active.unwrap_or(true),
            created_at: r
                .created_at
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
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
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, name, is_active"#,
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
        id: rule.id.to_string(),
        name: rule.name,
        is_active: rule.is_active.unwrap_or(true),
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
        r#"SELECT id FROM automation_rules WHERE id = $1 AND user_id = $2"#,
        rule_id,
        auth.user_id,
    )
    .fetch_optional(&state.db)
    .await?;

    if existing.is_none() {
        return Err(AppError::NotFound("Rule not found".into()));
    }

    sqlx::query!(
        r#"UPDATE automation_rules
           SET name = COALESCE($3, name),
               trigger_filter = COALESCE($4, trigger_filter),
               response_template = COALESCE($5, response_template),
               response_type = COALESCE($6, response_type),
               ai_model = COALESCE($7, ai_model),
               is_active = COALESCE($8, is_active),
               cooldown_minutes = COALESCE($9, cooldown_minutes),
               max_responses_per_hour = COALESCE($10, max_responses_per_hour),
               updated_at = NOW()
           WHERE id = $1 AND user_id = $2"#,
        rule_id,
        auth.user_id,
        request.name,
        request.trigger_filter,
        request.response_template,
        request.response_type,
        request.ai_model,
        request.is_active,
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
        r#"DELETE FROM automation_rules WHERE id = $1 AND user_id = $2"#,
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
        r#"SELECT id FROM automation_rules WHERE id = $1 AND user_id = $2"#,
        rule_id,
        auth.user_id,
    )
    .fetch_optional(&state.db)
    .await?;

    if existing.is_none() {
        return Err(AppError::NotFound("Rule not found".into()));
    }

    let logs = sqlx::query!(
        r#"SELECT id, trigger_id, trigger_type, response, status, error_message, created_at
           FROM automation_logs
           WHERE rule_id = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
        rule_id,
        query.limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to get logs: {e}")))?;

    let log_responses: Vec<LogEntryResponse> = logs
        .into_iter()
        .map(|l| LogEntryResponse {
            id: l.id.to_string(),
            trigger_id: l.trigger_id,
            trigger_type: l.trigger_type,
            response: l.response,
            status: l.status,
            error_message: l.error_message,
            created_at: l
                .created_at
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
        })
        .collect();

    let total = log_responses.len();

    Ok(Json(GetLogsResponse {
        logs: log_responses,
        total,
    }))
}
