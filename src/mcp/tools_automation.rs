// ─── MCP Automation Tools ───────────────────────────────────────
// Tools for managing automation rules and viewing execution logs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;

#[derive(Debug, sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    name: String,
    trigger_type: String,
    response_type: String,
    is_active: Option<bool>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateRuleInput {
    pub integration_id: String,
    pub name: String,
    pub trigger_type: String,
    pub trigger_filter: serde_json::Value,
    pub response_template: String,
    pub response_type: String,
    pub ai_model: Option<String>,
    pub cooldown_minutes: Option<i32>,
    pub max_responses_per_hour: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateRuleOutput {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListRulesInput {
    pub integration_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RuleInfo {
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    pub response_type: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListRulesOutput {
    pub rules: Vec<RuleInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateRuleInput {
    pub rule_id: String,
    pub name: Option<String>,
    pub trigger_filter: Option<serde_json::Value>,
    pub response_template: Option<String>,
    pub response_type: Option<String>,
    pub ai_model: Option<String>,
    pub is_active: Option<bool>,
    pub cooldown_minutes: Option<i32>,
    pub max_responses_per_hour: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteRuleInput {
    pub rule_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetLogsInput {
    pub rule_id: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LogEntry {
    pub id: String,
    pub trigger_id: String,
    pub trigger_type: String,
    pub response: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetLogsOutput {
    pub logs: Vec<LogEntry>,
    pub total: usize,
}

pub async fn create_rule(
    state: &AppState,
    input: &CreateRuleInput,
) -> Result<Json<CreateRuleOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id format")?;

    let cooldown = input.cooldown_minutes.unwrap_or(0);
    let max_per_hour = input.max_responses_per_hour.unwrap_or(10);

    let rule = sqlx::query!(
        r#"INSERT INTO automation_rules
           (user_id, integration_id, name, trigger_type, trigger_filter,
            response_template, response_type, ai_model, cooldown_minutes, max_responses_per_hour)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, name, is_active"#,
        user_id,
        integration_id,
        input.name,
        input.trigger_type,
        input.trigger_filter,
        input.response_template,
        input.response_type,
        input.ai_model,
        cooldown,
        max_per_hour,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to create rule: {e}"))?;

    Ok(Json(CreateRuleOutput {
        id: rule.id.to_string(),
        name: rule.name,
        is_active: rule.is_active.unwrap_or(true),
    }))
}

pub async fn list_rules(
    state: &AppState,
    input: &ListRulesInput,
) -> Result<Json<ListRulesOutput>, String> {
    let user_id = resolve_first_user(state).await?;

    let rules: Vec<RuleRow> = if let Some(ref integration_id_str) = input.integration_id {
        let integration_id = Uuid::parse_str(integration_id_str)
            .map_err(|_| "Invalid integration_id format")?;
        sqlx::query_as!(
            RuleRow,
            r#"SELECT id, name, trigger_type, response_type, is_active, created_at
               FROM automation_rules
               WHERE user_id = $1 AND integration_id = $2
               ORDER BY created_at DESC"#,
            user_id,
            integration_id,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Failed to list rules: {e}"))?
    } else {
        sqlx::query_as!(
            RuleRow,
            r#"SELECT id, name, trigger_type, response_type, is_active, created_at
               FROM automation_rules
               WHERE user_id = $1
               ORDER BY created_at DESC"#,
            user_id,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Failed to list rules: {e}"))?
    };

    let rule_infos: Vec<RuleInfo> = rules.into_iter().map(|r| RuleInfo {
        id: r.id.to_string(),
        name: r.name,
        trigger_type: r.trigger_type,
        response_type: r.response_type,
        is_active: r.is_active.unwrap_or(true),
        created_at: r.created_at.unwrap_or_else(chrono::Utc::now).to_rfc3339(),
    }).collect();

    let total = rule_infos.len();

    Ok(Json(ListRulesOutput { rules: rule_infos, total }))
}

pub async fn update_rule(
    state: &AppState,
    input: &UpdateRuleInput,
) -> Result<Json<crate::mcp::SuccessOutput>, String> {
    let rule_id = Uuid::parse_str(&input.rule_id)
        .map_err(|_| "Invalid rule_id format")?;

    sqlx::query!(
        r#"UPDATE automation_rules
           SET name = COALESCE($2, name),
               trigger_filter = COALESCE($3, trigger_filter),
               response_template = COALESCE($4, response_template),
               response_type = COALESCE($5, response_type),
               ai_model = COALESCE($6, ai_model),
               is_active = COALESCE($7, is_active),
               cooldown_minutes = COALESCE($8, cooldown_minutes),
               max_responses_per_hour = COALESCE($9, max_responses_per_hour),
               updated_at = NOW()
           WHERE id = $1"#,
        rule_id,
        input.name,
        input.trigger_filter,
        input.response_template,
        input.response_type,
        input.ai_model,
        input.is_active,
        input.cooldown_minutes,
        input.max_responses_per_hour,
    )
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to update rule: {e}"))?;

    Ok(Json(crate::mcp::SuccessOutput {
        success: true,
        message: "Rule updated".into(),
    }))
}

pub async fn delete_rule(
    state: &AppState,
    input: &DeleteRuleInput,
) -> Result<Json<crate::mcp::SuccessOutput>, String> {
    let rule_id = Uuid::parse_str(&input.rule_id)
        .map_err(|_| "Invalid rule_id format")?;

    sqlx::query!(
        r#"DELETE FROM automation_rules WHERE id = $1"#,
        rule_id,
    )
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to delete rule: {e}"))?;

    Ok(Json(crate::mcp::SuccessOutput {
        success: true,
        message: "Rule deleted".into(),
    }))
}

pub async fn get_logs(
    state: &AppState,
    input: &GetLogsInput,
) -> Result<Json<GetLogsOutput>, String> {
    let rule_id = Uuid::parse_str(&input.rule_id)
        .map_err(|_| "Invalid rule_id format")?;
    let limit = input.limit.unwrap_or(50);

    let logs = sqlx::query!(
        r#"SELECT id, trigger_id, trigger_type, response, status, error_message, created_at
           FROM automation_logs
           WHERE rule_id = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
        rule_id,
        limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to get logs: {e}"))?;

    let log_entries: Vec<LogEntry> = logs.into_iter().map(|l| LogEntry {
        id: l.id.to_string(),
        trigger_id: l.trigger_id,
        trigger_type: l.trigger_type,
        response: l.response,
        status: l.status,
        error_message: l.error_message,
        created_at: l.created_at.unwrap_or_else(chrono::Utc::now).to_rfc3339(),
    }).collect();

    let total = log_entries.len();

    Ok(Json(GetLogsOutput { logs: log_entries, total }))
}

use super::auth::resolve_first_user;
