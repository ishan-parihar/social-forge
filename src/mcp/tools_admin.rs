// ─── MCP Admin/Parity Tools ────────────────────────────────────
// MCP wrappers for REST endpoints that were missing from the MCP
// surface — identified in the v9 AI-agent-efficacy audit.
//
// Each tool here is a thin wrapper over an existing db::queries
// function or service call. The goal is MCP/CLI/API parity so an
// AI agent using only MCP can do everything the WebUI can do.
//
// NOTE: all SQL here uses runtime `sqlx::query()` (not the `query!`
// macro) to avoid requiring a live DB or .sqlx offline cache refresh
// at build time. Trade-off: no compile-time column type checking,
// but the queries are simple and the structs derive FromRow.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::db::queries;

// ── Recurring Posts ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PostsRepeatInput {
    /// Post ID (UUID)
    pub post_id: String,
    /// Repeat interval in days (e.g. 7 for weekly, 30 for monthly)
    pub interval_days: i32,
    /// End date for the repeat (RFC3339). Optional — if omitted, repeats indefinitely.
    pub end_date: Option<String>,
}

#[derive(Default, sqlx::FromRow)]
struct RepeatUpdate {
    id: Uuid,
    repeat_interval_days: Option<i32>,
    repeat_end_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Set up a recurring/evergreen post. The scheduler will create a
/// new queued copy of this post every `interval_days` days until
/// `end_date` (or forever if not specified).
pub async fn handle_posts_repeat(
    state: &AppState,
    input: &PostsRepeatInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.post_id)
        .map_err(|_| "Invalid post_id UUID".to_string())?;

    let end_date = match input.end_date.as_deref() {
        Some(d) if !d.is_empty() => {
            Some(chrono::DateTime::parse_from_rfc3339(d)
                .map_err(|e| format!("Invalid end_date (use RFC3339): {e}"))?
                .with_timezone(&chrono::Utc))
        }
        _ => None,
    };

    if input.interval_days < 1 {
        return Err("interval_days must be >= 1".into());
    }

    // Verify ownership
    let _post = queries::get_post(&state.db, post_id, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| "Post not found".to_string())?;

    let updated: RepeatUpdate = sqlx::query_as(
        "UPDATE posts
         SET repeat_interval_days = $1,
             repeat_end_date = $2,
             updated_at = NOW()
         WHERE id = $3 AND user_id = $4
         RETURNING id, repeat_interval_days, repeat_end_date",
    )
    .bind(input.interval_days)
    .bind(end_date)
    .bind(post_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to set repeat: {e}"))?;

    Ok(Json(serde_json::json!({
        "post_id": updated.id.to_string(),
        "repeat_interval_days": updated.repeat_interval_days,
        "repeat_end_date": updated.repeat_end_date.map(|d| d.to_rfc3339()),
        "message": format!("Post will repeat every {} days", input.interval_days),
    })))
}

// ── Post Tags ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PostsSetTagsInput {
    /// Post ID (UUID)
    pub post_id: String,
    /// List of tag IDs (UUIDs) to attach to this post. Replaces any existing tags.
    pub tag_ids: Vec<String>,
}

/// Attach tags to a post. Replaces any existing tags.
pub async fn handle_posts_set_tags(
    state: &AppState,
    input: &PostsSetTagsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.post_id)
        .map_err(|_| "Invalid post_id UUID".to_string())?;

    // Verify ownership
    let _post = queries::get_post(&state.db, post_id, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| "Post not found".to_string())?;

    // Parse tag IDs
    let tag_uuids: Vec<Uuid> = input.tag_ids.iter()
        .map(|s| Uuid::parse_str(s).map_err(|e| format!("Invalid tag UUID '{s}': {e}")))
        .collect::<Result<_, _>>()?;

    // Remove existing tags, then insert new ones
    sqlx::query("DELETE FROM post_tags WHERE post_id = $1")
        .bind(post_id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to clear existing tags: {e}"))?;

    for tag_id in &tag_uuids {
        sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(post_id)
            .bind(tag_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to attach tag: {e}"))?;
    }

    Ok(Json(serde_json::json!({
        "post_id": post_id.to_string(),
        "tag_ids": input.tag_ids,
        "attached": tag_uuids.len(),
    })))
}

// ── Media Delete ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MediaDeleteInput {
    /// Media ID (UUID)
    pub media_id: String,
}

/// Delete a media file. Removes the DB row AND the file on disk.
pub async fn handle_media_delete(
    state: &AppState,
    input: &MediaDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let media_id = Uuid::parse_str(&input.media_id)
        .map_err(|_| "Invalid media_id UUID".to_string())?;

    let entry = queries::delete_media(&state.db, media_id, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| "Media not found".to_string())?;

    // Delete the file from disk
    let upload_dir = std::path::Path::new(&state.config.media_dir);
    let filepath = upload_dir.join(&entry.storage_path);
    if filepath.exists() {
        if let Err(e) = tokio::fs::remove_file(&filepath).await {
            tracing::warn!("Failed to delete media file {filepath:?}: {e}");
        }
    }

    Ok(Json(serde_json::json!({
        "deleted": true,
        "media_id": media_id.to_string(),
    })))
}

// ── Integration Refresh ───────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IntegrationsRefreshInput {
    /// Integration ID (UUID)
    pub integration_id: String,
}

/// Manually refresh the OAuth access token for an integration.
/// Useful when the proactive refresh cycle hasn't run yet but the
/// token is known to be expiring soon.
pub async fn handle_integrations_refresh(
    state: &AppState,
    input: &IntegrationsRefreshInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id UUID".to_string())?;

    let integration = queries::get_integration(&state.db, integration_id, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| "Integration not found".to_string())?;

    let provider = state.providers.get(&integration.provider_identifier)
        .ok_or_else(|| format!("Provider '{}' not registered", integration.provider_identifier))?;

    // Decrypt the refresh token if encryption is configured
    let refresh_token = crate::crypto::maybe_decrypt_token(
        integration.refresh_token.as_deref().unwrap_or(""),
        state.token_key.as_ref(),
    );

    let token = provider.refresh_token(&refresh_token).await
        .map_err(|e| format!("Token refresh failed: {e}"))?;

    // Encrypt the new access token before storing
    let enc_access_token = if let Some(ref k) = state.token_key {
        crate::crypto::encrypt_string(&token.access_token, k)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to encrypt refreshed token: {e}");
                token.access_token.clone()
            })
    } else {
        token.access_token.clone()
    };

    queries::update_integration_token(
        &state.db,
        integration_id,
        &enc_access_token,
        token.refresh_token.as_deref(),
        token.expires_in.map(|e| chrono::Utc::now() + chrono::Duration::seconds(e as i64)),
    )
    .await
    .map_err(|e| format!("Failed to save refreshed token: {e}"))?;

    // Clear the refresh_needed flag
    let _ = sqlx::query("UPDATE integrations SET refresh_needed = false WHERE id = $1")
        .bind(integration_id)
        .execute(&state.db)
        .await;

    Ok(Json(serde_json::json!({
        "integration_id": integration_id.to_string(),
        "refreshed": true,
        "expires_in": token.expires_in,
    })))
}

// ── Signatures ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SignatureCreateInput {
    /// Signature name (for identification)
    pub name: String,
    /// Signature content (appended to posts)
    pub content: String,
    /// Provider identifier (e.g. "x", "linkedin"). Optional — if omitted, applies to all.
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SignatureListInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SignatureUpdateInput {
    pub signature_id: String,
    pub name: Option<String>,
    pub content: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SignatureDeleteInput {
    pub signature_id: String,
}

#[derive(Default, sqlx::FromRow)]
struct SignatureRow {
    id: Uuid,
    name: String,
    content: String,
    provider: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn handle_signatures_list(
    state: &AppState,
    _input: &SignatureListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let sigs: Vec<SignatureRow> = sqlx::query_as(
        "SELECT id, name, content, provider, created_at, updated_at
         FROM signatures WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let data: Vec<_> = sigs.iter().map(|s| serde_json::json!({
        "id": s.id.to_string(),
        "name": s.name,
        "content": s.content,
        "provider": s.provider,
        "created_at": s.created_at.to_rfc3339(),
        "updated_at": s.updated_at.to_rfc3339(),
    })).collect();

    Ok(Json(serde_json::json!({ "signatures": data })))
}

pub async fn handle_signatures_create(
    state: &AppState,
    input: &SignatureCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let sig: SignatureRow = sqlx::query_as(
        "INSERT INTO signatures (user_id, name, content, provider)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, content, provider, created_at, updated_at",
    )
    .bind(user_id)
    .bind(&input.name)
    .bind(&input.content)
    .bind(&input.provider)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    Ok(Json(serde_json::json!({
        "id": sig.id.to_string(),
        "name": sig.name,
        "content": sig.content,
        "provider": sig.provider,
        "created_at": sig.created_at.to_rfc3339(),
    })))
}

pub async fn handle_signatures_update(
    state: &AppState,
    input: &SignatureUpdateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let sig_id = Uuid::parse_str(&input.signature_id)
        .map_err(|_| "Invalid signature_id UUID".to_string())?;

    let sig: SignatureRow = sqlx::query_as(
        "UPDATE signatures
         SET name = COALESCE($2, name),
             content = COALESCE($3, content),
             provider = COALESCE($4, provider),
             updated_at = NOW()
         WHERE id = $1 AND user_id = $5
         RETURNING id, name, content, provider, created_at, updated_at",
    )
    .bind(sig_id)
    .bind(&input.name)
    .bind(&input.content)
    .bind(&input.provider)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    Ok(Json(serde_json::json!({
        "id": sig.id.to_string(),
        "name": sig.name,
        "content": sig.content,
        "provider": sig.provider,
    })))
}

pub async fn handle_signatures_delete(
    state: &AppState,
    input: &SignatureDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let sig_id = Uuid::parse_str(&input.signature_id)
        .map_err(|_| "Invalid signature_id UUID".to_string())?;

    sqlx::query("DELETE FROM signatures WHERE id = $1 AND user_id = $2")
        .bind(sig_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    Ok(Json(serde_json::json!({ "deleted": true, "id": input.signature_id })))
}

// ── Analytics Summary ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyticsSummaryInput {
    /// Number of days to include in the summary (default 30)
    pub days: Option<i32>,
}

#[derive(Default, sqlx::FromRow)]
struct SummaryCounts {
    total: i64,
    published: i64,
    failed: i64,
    draft: i64,
    queued: i64,
}

/// Get aggregate analytics summary across all platforms: total posts,
/// published/failed/draft/queued counts, posts by provider, posts by day.
pub async fn handle_analytics_summary(
    state: &AppState,
    input: &AnalyticsSummaryInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let days = input.days.unwrap_or(30).max(1).min(365) as i64;

    let counts: SummaryCounts = sqlx::query_as(
        "SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE state = 'published') as published,
            COUNT(*) FILTER (WHERE state = 'error') as failed,
            COUNT(*) FILTER (WHERE state = 'draft') as draft,
            COUNT(*) FILTER (WHERE state = 'queued') as queued
           FROM posts
           WHERE user_id = $1
             AND created_at > NOW() - make_interval(secs => $2::double precision * 86400.0)",
    )
    .bind(user_id)
    .bind(days as f64)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    Ok(Json(serde_json::json!({
        "total_posts": counts.total,
        "published": counts.published,
        "failed": counts.failed,
        "draft": counts.draft,
        "queued": counts.queued,
        "days": days,
    })))
}

// ── Feed Analytics ────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FeedAnalyticsInput {
    /// Number of days to include (default 30)
    pub days: Option<i32>,
}

#[derive(Default, sqlx::FromRow)]
struct FeedTotals {
    total_posts: i64,
    total_likes: i64,
    total_comments: i64,
    total_shares: i64,
    total_impressions: i64,
}

/// Get feed-level analytics: engagement totals across all imported
/// feed posts, top posts by engagement, engagement trends over time.
pub async fn handle_feed_analytics(
    state: &AppState,
    input: &FeedAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let _user_id = super::tools_posts::resolve_first_user(state).await?;
    let days = input.days.unwrap_or(30).max(1).min(365) as i64;

    let totals: FeedTotals = sqlx::query_as(
        "SELECT
            COUNT(*) as total_posts,
            COALESCE(SUM(likes), 0) as total_likes,
            COALESCE(SUM(comments), 0) as total_comments,
            COALESCE(SUM(shares), 0) as total_shares,
            COALESCE(SUM(impressions), 0) as total_impressions
           FROM post_engagement
           WHERE updated_at > NOW() - make_interval(secs => $1::double precision * 86400.0)",
    )
    .bind(days as f64)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    Ok(Json(serde_json::json!({
        "total_posts": totals.total_posts,
        "total_likes": totals.total_likes,
        "total_comments": totals.total_comments,
        "total_shares": totals.total_shares,
        "total_impressions": totals.total_impressions,
        "days": days,
    })))
}
