// ─── Database Queries ─────────────────────────────────────────
// Typed SQL query functions using sqlx.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::models::*;
use super::PgPool;

// ══════════════════════════════════════════════════════════════
// USERS
// ══════════════════════════════════════════════════════════════

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "INSERT INTO users (email, password, name) VALUES ($1, $2, $3)
         RETURNING id, email, password, name, timezone, created_at, updated_at",
        email,
        password_hash,
        name,
    )
    .fetch_one(pool)
    .await
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, email, password, name, timezone, created_at, updated_at
         FROM users WHERE email = $1",
        email,
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, email, password, name, timezone, created_at, updated_at
         FROM users WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await
}

// ══════════════════════════════════════════════════════════════
// INTEGRATIONS
// ══════════════════════════════════════════════════════════════

pub async fn create_integration(
    pool: &PgPool,
    user_id: Uuid,
    provider_identifier: &str,
    provider_name: &str,
    internal_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: Option<DateTime<Utc>>,
    profile_name: Option<&str>,
    profile_picture: Option<&str>,
    profile_url: Option<&str>,
    root_internal_id: Option<&str>,
) -> Result<Integration, sqlx::Error> {
    sqlx::query_as!(
        Integration,
        r#"INSERT INTO integrations
           (user_id, provider_identifier, provider_name, internal_id,
            access_token, refresh_token, token_expires_at,
            profile_name, profile_picture, profile_url,
            root_internal_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           ON CONFLICT (user_id, provider_identifier, internal_id)
           DO UPDATE SET access_token = $5, refresh_token = $6,
             token_expires_at = $7, profile_name = $8,
             profile_picture = $9, profile_url = $10,
             root_internal_id = COALESCE($11, integrations.root_internal_id),
             refresh_needed = false, disabled = false,
             updated_at = now()
           RETURNING id, user_id, provider_identifier, provider_name,
             internal_id, access_token, refresh_token, token_expires_at,
             profile_name, profile_picture, profile_url, disabled,
             refresh_needed, root_internal_id, posting_times, created_at, updated_at"#,
        user_id,
        provider_identifier,
        provider_name,
        internal_id,
        access_token,
        refresh_token,
        token_expires_at,
        profile_name,
        profile_picture,
        profile_url,
        root_internal_id,
    )
    .fetch_one(pool)
    .await
}

pub async fn list_integrations(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Integration>, sqlx::Error> {
    sqlx::query_as!(
        Integration,
        "SELECT id, user_id, provider_identifier, provider_name, internal_id,
                access_token, refresh_token, token_expires_at,
                profile_name, profile_picture, profile_url, disabled,
                refresh_needed, root_internal_id, posting_times, created_at, updated_at
         FROM integrations WHERE user_id = $1 ORDER BY created_at DESC",
        user_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_integration(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Integration>, sqlx::Error> {
    sqlx::query_as!(
        Integration,
        "SELECT id, user_id, provider_identifier, provider_name, internal_id,
                access_token, refresh_token, token_expires_at,
                profile_name, profile_picture, profile_url, disabled,
                refresh_needed, root_internal_id, posting_times, created_at, updated_at
         FROM integrations WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_integration_token(
    pool: &PgPool,
    id: Uuid,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE integrations SET access_token = $1, refresh_token = $2,
         token_expires_at = $3, refresh_needed = false, updated_at = now()
         WHERE id = $4",
        access_token,
        refresh_token,
        token_expires_at,
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_integration(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query!(
        "DELETE FROM integrations WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

// ══════════════════════════════════════════════════════════════
// POSTS
// ══════════════════════════════════════════════════════════════

pub async fn create_post(
    pool: &PgPool,
    user_id: Uuid,
    integration_id: Uuid,
    content: &str,
    title: Option<&str>,
    media: &serde_json::Value,
    settings: &serde_json::Value,
    scheduled_at: Option<DateTime<Utc>>,
    state: Option<PostState>,
) -> Result<Post, sqlx::Error> {
    let st = state.unwrap_or(PostState::Draft);
    sqlx::query_as!(
        Post,
        r#"INSERT INTO posts
           (user_id, integration_id, content, title, media, settings, scheduled_at, state)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, user_id, integration_id, state as "state: PostState",
             content, title, media, settings, scheduled_at, published_at,
             platform_post_id, platform_post_url, error_message,
             created_at, updated_at"#,
        user_id,
        integration_id,
        content,
        title,
        media,
        settings,
        scheduled_at,
        st as PostState,
    )
    .fetch_one(pool)
    .await
}

pub async fn list_posts(
    pool: &PgPool,
    user_id: Uuid,
    state_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Post>, sqlx::Error> {
    if let Some(st) = state_filter {
        let ps: PostState = match st {
            "draft" => PostState::Draft,
            "queued" => PostState::Queued,
            "published" => PostState::Published,
            "error" => PostState::Error,
            _ => return list_posts_all(pool, user_id, limit, offset).await,
        };
        sqlx::query_as!(
            Post,
            r#"SELECT id, user_id, integration_id, state as "state: PostState",
               content, title, media, settings, scheduled_at, published_at,
               platform_post_id, platform_post_url, error_message,
               created_at, updated_at
             FROM posts WHERE user_id = $1 AND state = $2
             ORDER BY scheduled_at DESC NULLS LAST, created_at DESC
             LIMIT $3 OFFSET $4"#,
            user_id,
            ps as PostState,
            limit,
            offset,
        )
        .fetch_all(pool)
        .await
    } else {
        list_posts_all(pool, user_id, limit, offset).await
    }
}

/// Count posts for a user, optionally filtered by state
pub async fn count_posts_by_user(
    pool: &PgPool,
    user_id: Uuid,
    state_filter: Option<&str>,
) -> Result<i64, sqlx::Error> {
    if let Some(st) = state_filter {
        let ps: PostState = match st {
            "draft" => PostState::Draft,
            "queued" => PostState::Queued,
            "published" => PostState::Published,
            "error" => PostState::Error,
            _ => {
                let row: (Option<i64>,) = sqlx::query_as(
                    "SELECT COUNT(*)::bigint FROM posts WHERE user_id = $1"
                )
                .bind(user_id)
                .fetch_one(pool)
                .await?;
                return Ok(row.0.unwrap_or(0));
            }
        };
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM posts WHERE user_id = $1 AND state = $2"
        )
        .bind(user_id)
        .bind(ps)
        .fetch_one(pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    } else {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM posts WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    }
}

async fn list_posts_all(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
           content, title, media, settings, scheduled_at, published_at,
           platform_post_id, platform_post_url, error_message,
           created_at, updated_at
         FROM posts WHERE user_id = $1
         ORDER BY scheduled_at DESC NULLS LAST, created_at DESC
         LIMIT $2 OFFSET $3"#,
        user_id,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_post(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
           content, title, media, settings, scheduled_at, published_at,
           platform_post_id, platform_post_url, error_message,
           created_at, updated_at
         FROM posts WHERE id = $1 AND user_id = $2"#,
        id,
        user_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_post_content(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    content: &str,
    title: Option<&str>,
    media: &serde_json::Value,
    settings: &serde_json::Value,
) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"UPDATE posts SET content = $1, title = $2, media = $3, settings = $4,
           updated_at = now()
           WHERE id = $5 AND user_id = $6
           RETURNING id, user_id, integration_id, state as "state: PostState",
             content, title, media, settings, scheduled_at, published_at,
             platform_post_id, platform_post_url, error_message,
             created_at, updated_at"#,
        content,
        title,
        media,
        settings,
        id,
        user_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn schedule_post(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    scheduled_at: DateTime<Utc>,
) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"UPDATE posts SET scheduled_at = $1, state = 'queued',
           updated_at = now()
           WHERE id = $2 AND user_id = $3
           RETURNING id, user_id, integration_id, state as "state: PostState",
             content, title, media, settings, scheduled_at, published_at,
             platform_post_id, platform_post_url, error_message,
             created_at, updated_at"#,
        scheduled_at,
        id,
        user_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_post_state(
    pool: &PgPool,
    id: Uuid,
    state: PostState,
    platform_post_id: Option<&str>,
    platform_post_url: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = if state == PostState::Published {
        Some(Utc::now())
    } else {
        None
    };
    sqlx::query!(
        r#"UPDATE posts SET state = $1, published_at = COALESCE($2, published_at),
           platform_post_id = COALESCE($3, platform_post_id),
           platform_post_url = COALESCE($4, platform_post_url),
           error_message = $5, updated_at = now()
           WHERE id = $6"#,
        state as PostState,
        now,
        platform_post_id,
        platform_post_url,
        error_message,
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_post(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query!("DELETE FROM posts WHERE id = $1 AND user_id = $2", id, user_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Get a single post with its integration details (used for retry/publish now)
pub async fn get_post_with_integration(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
) -> Result<Option<PostWithIntegration>, sqlx::Error> {
    sqlx::query_as!(
        PostWithIntegration,
        r#"SELECT p.id, p.user_id, p.integration_id,
           p.state as "state: PostState",
           p.content, p.title, p.media, p.settings,
           p.scheduled_at, p.published_at,
           p.platform_post_id, p.platform_post_url, p.error_message,
           p.created_at, p.updated_at,
           i.provider_identifier, i.access_token,
           i.refresh_token, i.token_expires_at,
           i.disabled as "integration_disabled",
           i.refresh_needed as "integration_refresh_needed"
         FROM posts p
         JOIN integrations i ON p.integration_id = i.id
         WHERE p.id = $1 AND p.user_id = $2"#,
        post_id,
        user_id,
    )
    .fetch_optional(pool)
    .await
}

/// Get posts due for publishing
pub async fn get_due_posts(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PostWithIntegration>, sqlx::Error> {
    sqlx::query_as!(
        PostWithIntegration,
        r#"SELECT p.id, p.user_id, p.integration_id,
           p.state as "state: PostState",
           p.content, p.title, p.media, p.settings,
           p.scheduled_at, p.published_at,
           p.platform_post_id, p.platform_post_url, p.error_message,
           p.created_at, p.updated_at,
           i.provider_identifier, i.access_token,
           i.refresh_token, i.token_expires_at,
           i.disabled as "integration_disabled",
           i.refresh_needed as "integration_refresh_needed"
         FROM posts p
         JOIN integrations i ON p.integration_id = i.id
         WHERE p.state = 'queued'
           AND p.scheduled_at <= NOW()
           AND i.disabled = false
         ORDER BY p.scheduled_at ASC
         LIMIT $1"#,
        limit,
    )
    .fetch_all(pool)
    .await
}

/// Get posts for a date range (calendar view)
pub async fn get_posts_by_date_range(
    pool: &PgPool,
    user_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
           content, title, media, settings, scheduled_at, published_at,
           platform_post_id, platform_post_url, error_message,
           created_at, updated_at
         FROM posts
         WHERE user_id = $1
           AND scheduled_at IS NOT NULL
           AND scheduled_at >= $2
           AND scheduled_at <= $3
         ORDER BY scheduled_at ASC"#,
        user_id,
        start,
        end,
    )
    .fetch_all(pool)
    .await
}

/// Find next free time slot for scheduling
pub async fn find_next_free_slot(
    pool: &PgPool,
    user_id: Uuid,
    integration_id: Option<Uuid>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    // Always use the per-integration query (when integration_id is None,
    // check all posts for this user)
    let last: Option<DateTime<Utc>> = if let Some(iid) = integration_id {
        sqlx::query_scalar!(
            r#"SELECT MAX(scheduled_at) FROM posts
               WHERE user_id = $1 AND integration_id = $2 AND state != 'error'"#,
            user_id,
            iid
        )
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar!(
            r#"SELECT MAX(scheduled_at) FROM posts
               WHERE user_id = $1 AND state != 'error'"#,
            user_id,
        )
        .fetch_one(pool)
        .await?
    };

    Ok(Some(last.unwrap_or_else(Utc::now) + chrono::Duration::hours(2)))
}

// ══════════════════════════════════════════════════════════════
// MEDIA
// ══════════════════════════════════════════════════════════════

pub async fn create_media(
    pool: &PgPool,
    user_id: Uuid,
    original_name: &str,
    storage_path: &str,
    mime_type: &str,
    file_size: i64,
    width: Option<i32>,
    height: Option<i32>,
) -> Result<MediaEntry, sqlx::Error> {
    sqlx::query_as!(
        MediaEntry,
        "INSERT INTO media (user_id, original_name, storage_path, mime_type, file_size, width, height)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, user_id, original_name, storage_path, mime_type,
           file_size, width, height, created_at",
        user_id,
        original_name,
        storage_path,
        mime_type,
        file_size,
        width,
        height,
    )
    .fetch_one(pool)
    .await
}

pub async fn list_media(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaEntry>, sqlx::Error> {
    sqlx::query_as!(
        MediaEntry,
        "SELECT id, user_id, original_name, storage_path, mime_type,
                file_size, width, height, created_at
         FROM media WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
        user_id,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_media(pool: &PgPool, id: Uuid) -> Result<Option<MediaEntry>, sqlx::Error> {
    sqlx::query_as!(
        MediaEntry,
        "SELECT id, user_id, original_name, storage_path, mime_type,
                file_size, width, height, created_at
         FROM media WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_media_user(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<MediaEntry>, sqlx::Error> {
    sqlx::query_as!(
        MediaEntry,
        "SELECT id, user_id, original_name, storage_path, mime_type,
                file_size, width, height, created_at
         FROM media WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .fetch_optional(pool)
    .await
}

// ══════════════════════════════════════════════════════════════
// OAUTH STATE
// ══════════════════════════════════════════════════════════════

pub async fn save_oauth_state(
    pool: &PgPool,
    state: &str,
    provider: &str,
    code_verifier: &str,
    redirect_uri: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO oauth_states (state, provider, code_verifier, redirect_uri)
         VALUES ($1, $2, $3, $4)",
        state,
        provider,
        code_verifier,
        redirect_uri,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_oauth_state(pool: &PgPool, state: &str) -> Result<Option<OAuthState>, sqlx::Error> {
    sqlx::query_as!(
        OAuthState,
        "SELECT id, state, provider, code_verifier, redirect_uri, created_at, expires_at
         FROM oauth_states WHERE state = $1 AND expires_at > NOW()",
        state,
    )
    .fetch_optional(pool)
    .await
}

pub async fn delete_oauth_state(pool: &PgPool, state: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM oauth_states WHERE state = $1", state)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn cleanup_expired_oauth_states(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query!("DELETE FROM oauth_states WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}
