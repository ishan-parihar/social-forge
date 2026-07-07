// ─── Database Queries ─────────────────────────────────────────
// Typed SQL query functions using sqlx.

use chrono::{DateTime, Utc};
use serde::Serialize;
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

pub async fn create_user_with_id(
    pool: &PgPool,
    id: Uuid,
    email: &str,
    password_hash: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "INSERT INTO users (id, email, password, name) VALUES ($1, $2, $3, $4)
         ON CONFLICT (email) DO UPDATE SET id = EXCLUDED.id, password = EXCLUDED.password, name = EXCLUDED.name
         RETURNING id, email, password, name, timezone, created_at, updated_at",
        id,
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
    auth_method: Option<&str>,
) -> Result<Integration, sqlx::Error> {
    let method = auth_method.unwrap_or("oauth");
    sqlx::query_as!(
        Integration,
        r#"INSERT INTO integrations
           (user_id, provider_identifier, provider_name, internal_id,
            access_token, refresh_token, token_expires_at,
            profile_name, profile_picture, profile_url,
            root_internal_id, auth_method)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           ON CONFLICT (user_id, provider_identifier, internal_id)
           DO UPDATE SET access_token = $5, refresh_token = $6,
             token_expires_at = $7, profile_name = $8,
             profile_picture = $9, profile_url = $10,
             root_internal_id = COALESCE($11, integrations.root_internal_id),
             auth_method = $12,
             refresh_needed = false, disabled = false,
             updated_at = now()
           RETURNING id, user_id, provider_identifier, provider_name,
             internal_id, access_token, refresh_token, token_expires_at,
             profile_name, profile_picture, profile_url, disabled,
             refresh_needed, root_internal_id, posting_times, auth_method, created_at, updated_at"#,
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
        method,
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
                refresh_needed, root_internal_id, posting_times, auth_method, created_at, updated_at
         FROM integrations WHERE user_id = $1 ORDER BY created_at DESC",
        user_id,
    )
    .fetch_all(pool)
    .await
}

/// List all non-disabled integrations across all users
pub async fn list_all_integrations_across_users(
    pool: &PgPool,
) -> Result<Vec<Integration>, sqlx::Error> {
    sqlx::query_as!(
        Integration,
        "SELECT id, user_id, provider_identifier, provider_name, internal_id,
                access_token, refresh_token, token_expires_at,
                profile_name, profile_picture, profile_url, disabled,
                refresh_needed, root_internal_id, posting_times, auth_method, created_at, updated_at
         FROM integrations WHERE disabled = false ORDER BY user_id, provider_identifier",
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
                refresh_needed, root_internal_id, posting_times, auth_method, created_at, updated_at
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

/// Mark an integration as needing reconnection (e.g. scope mismatch, revoked token).
pub async fn mark_integration_refresh_needed(
    pool: &PgPool,
    id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE integrations SET refresh_needed = true, updated_at = now() WHERE id = $1",
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get integrations that need proactive token refresh (tokens expiring within `window_hours`)
pub async fn get_integrations_needing_refresh(
    pool: &PgPool,
    provider_identifier: &str,
    window_hours: i64,
) -> Result<Vec<Integration>, sqlx::Error> {
    let cutoff_time = chrono::Utc::now() + chrono::Duration::hours(window_hours);
    sqlx::query_as!(
        Integration,
        r#"SELECT id, user_id, provider_identifier, provider_name, internal_id,
                access_token, refresh_token, token_expires_at,
                profile_name, profile_picture, profile_url, disabled,
                refresh_needed, root_internal_id, posting_times, auth_method, created_at, updated_at
         FROM integrations
         WHERE provider_identifier = $1
           AND disabled = false
           AND refresh_needed = false
           AND refresh_token IS NOT NULL
           AND token_expires_at IS NOT NULL
           AND token_expires_at <= $2"#,
        provider_identifier,
        cutoff_time,
    )
    .fetch_all(pool)
    .await
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

/// Batch-create posts for multiple integrations in a single transaction.
pub async fn create_posts_for_integrations(
    pool: &PgPool,
    user_id: Uuid,
    integration_ids: &[Uuid],
    content: &str,
    title: Option<&str>,
    media: &serde_json::Value,
    settings: &serde_json::Value,
    scheduled_at: Option<DateTime<Utc>>,
    state: Option<PostState>,
    first_comment: Option<&str>,
    sequence: i32,
) -> Result<Vec<Post>, sqlx::Error> {
    let st = state.unwrap_or(PostState::Draft);
    let mut tx = pool.begin().await?;
    let mut posts = Vec::with_capacity(integration_ids.len());

    for &integration_id in integration_ids {
        let post = sqlx::query_as::<_, Post>(
            r#"INSERT INTO posts
               (user_id, integration_id, content, title, media, settings, scheduled_at, state, first_comment, sequence)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING id, user_id, integration_id, state,
                 content, title, media, settings, scheduled_at, published_at,
                 platform_post_id, platform_post_url, error_message,
                 created_at, updated_at,
                 repeat_interval_days, repeat_end_date, group_id,
                 first_comment, sequence, idempotency_key"#,
        )
        .bind(user_id)
        .bind(integration_id)
        .bind(content)
        .bind(title)
        .bind(media)
        .bind(settings)
        .bind(scheduled_at)
        .bind(&st)
        .bind(first_comment)
        .bind(sequence)
        .fetch_one(&mut *tx)
        .await?;
        posts.push(post);
    }

    tx.commit().await?;
    Ok(posts)
}

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
    first_comment: Option<&str>,
    sequence: i32,
) -> Result<Post, sqlx::Error> {
    let st = state.unwrap_or(PostState::Draft);
    // Runtime query (Phase v22 — idempotency_key column added, can't
    // regenerate .sqlx offline cache without a live Postgres).
    sqlx::query_as::<_, Post>(
        r#"INSERT INTO posts
           (user_id, integration_id, content, title, media, settings, scheduled_at, state, first_comment, sequence)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, user_id, integration_id, state as "state: PostState",
              content, title, media, settings, scheduled_at, published_at,
              platform_post_id, platform_post_url, error_message,
              created_at, updated_at,
              repeat_interval_days, repeat_end_date, group_id,
              first_comment, sequence, idempotency_key"#,
    )
    .bind(user_id)
    .bind(integration_id)
    .bind(content)
    .bind(title)
    .bind(media)
    .bind(settings)
    .bind(scheduled_at)
    .bind(st)
    .bind(first_comment)
    .bind(sequence)
    .fetch_one(pool)
    .await
}

/// Create thread posts (multiple content parts sharing a group_id)
pub async fn create_thread_posts(
    pool: &PgPool,
    user_id: Uuid,
    integration_ids: &[Uuid],
    content_parts: &[String],
    scheduled_at: Option<DateTime<Utc>>,
    state: Option<PostState>,
    group_id: Uuid,
    delay_minutes: Option<i64>,
) -> Result<Vec<Post>, sqlx::Error> {
    let st = state.unwrap_or(PostState::Draft);
    let mut tx = pool.begin().await?;
    let mut posts = Vec::new();
    let mut seq = 1i32;

    for part in content_parts {
        // Apply per-part delay: each part's scheduled_at is offset by
        // (seq - 1) * delay_minutes. Part 1 publishes at scheduled_at,
        // part 2 at scheduled_at + delay, part 3 at scheduled_at + 2*delay, etc.
        let part_scheduled_at = scheduled_at.map(|base| {
            if let Some(delay) = delay_minutes {
                if delay > 0 && seq > 1 {
                    base + chrono::Duration::minutes(delay * (seq as i64 - 1))
                } else {
                    base
                }
            } else {
                base
            }
        });

        for &integration_id in integration_ids {
            let post = sqlx::query_as::<_, Post>(
                r#"INSERT INTO posts
                   (user_id, integration_id, content, title, media, settings, scheduled_at, state, first_comment, sequence, group_id)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                   RETURNING id, user_id, integration_id, state,
                     content, title, media, settings, scheduled_at, published_at,
                     platform_post_id, platform_post_url, error_message,
                     created_at, updated_at,
                     repeat_interval_days, repeat_end_date, group_id,
                     first_comment, sequence, idempotency_key"#,
            )
            .bind(user_id)
            .bind(integration_id)
            .bind(part)
            .bind(None::<&str>)
            .bind(&serde_json::Value::Null)
            .bind(&serde_json::json!({}))
            .bind(part_scheduled_at)
            .bind(&st)
            .bind(None::<&str>)
            .bind(seq)
            .bind(group_id)
            .fetch_one(&mut *tx)
            .await?;
            posts.push(post);
        }
        seq += 1;
    }

    tx.commit().await?;
    Ok(posts)
}

/// Get all posts sharing a group_id (for thread display)
pub async fn get_posts_by_group_id(
    pool: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as::<_, Post>(
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
           content, title, media, settings, scheduled_at, published_at,
           platform_post_id, platform_post_url, error_message,
           created_at, updated_at,
           repeat_interval_days, repeat_end_date, group_id,
           first_comment, sequence, idempotency_key
         FROM posts WHERE user_id = $1 AND group_id = $2
         ORDER BY sequence ASC, created_at ASC"#,
    )
        .bind(user_id)
        .bind(group_id)
    .fetch_all(pool)
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
            "idea" => PostState::Idea,
            "draft" => PostState::Draft,
            "queued" => PostState::Queued,
            "published" => PostState::Published,
            "error" => PostState::Error,
            _ => return list_posts_all(pool, user_id, limit, offset).await,
        };
        // Runtime query (not query_as!) so we don't need to regenerate the
        // sqlx offline cache for this minor WHERE-clause change. The query
        // is identical to the cached version except for the added
        // `AND deleted_at IS NULL` filter.
        sqlx::query_as::<_, Post>(
            r#"SELECT id, user_id, integration_id, state as "state: PostState",
               content, title, media, settings, scheduled_at, published_at,
               platform_post_id, platform_post_url, error_message,
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence, idempotency_key
             FROM posts WHERE user_id = $1 AND state = $2 AND deleted_at IS NULL
             ORDER BY scheduled_at DESC NULLS LAST, created_at DESC
             LIMIT $3 OFFSET $4"#,
        )
        .bind(user_id)
        .bind(ps)
        .bind(limit)
        .bind(offset)
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
            "idea" => PostState::Idea,
            "draft" => PostState::Draft,
            "queued" => PostState::Queued,
            "published" => PostState::Published,
            "error" => PostState::Error,
            _ => {
                let row: (Option<i64>,) = sqlx::query_as(
                    "SELECT COUNT(*)::bigint FROM posts WHERE user_id = $1 AND deleted_at IS NULL"
                )
                .bind(user_id)
                .fetch_one(pool)
                .await?;
                return Ok(row.0.unwrap_or(0));
            }
        };
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM posts WHERE user_id = $1 AND state = $2 AND deleted_at IS NULL"
        )
        .bind(user_id)
        .bind(ps)
        .fetch_one(pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    } else {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM posts WHERE user_id = $1 AND deleted_at IS NULL"
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
    // Runtime query (see note in list_posts above).
    sqlx::query_as::<_, Post>(
         r#"SELECT id, user_id, integration_id, state as "state: PostState",
            content, title, media, settings, scheduled_at, published_at,
            platform_post_id, platform_post_url, error_message,
            created_at, updated_at,
            repeat_interval_days, repeat_end_date, group_id,
            first_comment, sequence, idempotency_key
          FROM posts WHERE user_id = $1 AND deleted_at IS NULL
          ORDER BY scheduled_at DESC NULLS LAST, created_at DESC
          LIMIT $2 OFFSET $3"#,
     )
     .bind(user_id)
     .bind(limit)
     .bind(offset)
     .fetch_all(pool)
     .await
 }

// ── Phase 5: search + filter + sort variants ───────────────────
// Runtime queries (not sqlx::query! macros) per AGENTS.md §0 rule 4.
// These support the new ListPostsQuery params: q, integration_ids,
// tag_ids, sort. The original list_posts / list_posts_all are kept
// for backward compatibility (MCP/CLI use them).

/// Build the ORDER BY clause from a sort string.
/// Supported: "scheduled_date" (default), "created_date", "engagement".
/// Prefix with "-" for descending (descending is the default for all).
fn sort_to_order_by(sort: &str) -> &'static str {
    let ascending = !sort.starts_with('-');
    let field = sort.trim_start_matches('-');
    // We only support descending for now (ascending would need index work);
    // ignore the ascending flag and always return DESC.
    let _ = ascending;
    match field {
        "created_date" => "created_at DESC NULLS LAST",
        "engagement" => {
            // engagement sort: left-join post_engagement and order by
            // (likes + comments + shares) DESC. Done in the query itself.
            "engagement DESC NULLS LAST"
        },
        _ => "scheduled_at DESC NULLS LAST, created_at DESC",
    }
}

/// Search + filter + sort posts for a user. Used by the Phase 5 posts
/// list endpoint. All params are optional; passing None for a filter
/// means "no filter on this field".
pub async fn list_posts_search(
    pool: &PgPool,
    user_id: Uuid,
    state_filter: Option<&str>,
    q: Option<&str>,
    integration_ids: Option<&[Uuid]>,
    tag_ids: Option<&[Uuid]>,
    sort: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Post>, sqlx::Error> {
    let order_by = sort_to_order_by(sort);

    // Build a dynamic query. We use string interpolation for the ORDER BY
    // (safe because sort_to_order_by returns a fixed set of literals) and
    // bind all user-provided values as parameters.
    let q_pattern = q.map(|s| {
        let escaped = s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        format!("%{escaped}%")
    });

    let sql: String = if sort == "engagement" {
        // Engagement sort needs a LEFT JOIN to post_engagement.
        r#"SELECT p.id, p.user_id, p.integration_id, p.state as "state: PostState",
                  p.content, p.title, p.media, p.settings, p.scheduled_at, p.published_at,
                  p.platform_post_id, p.platform_post_url, p.error_message,
                  p.created_at, p.updated_at,
                  p.repeat_interval_days, p.repeat_end_date, p.group_id,
                  p.first_comment, p.sequence, p.idempotency_key
           FROM posts p
           LEFT JOIN post_engagement pe ON pe.post_id = p.id
           WHERE p.user_id = $1
             AND p.deleted_at IS NULL
             AND ($2::text IS NULL OR p.state = $2::text)
             AND ($3::text IS NULL OR p.content ILIKE $3 OR p.title ILIKE $3)
             AND ($4::uuid[] IS NULL OR p.integration_id = ANY($4::uuid[]))
             AND ($5::uuid[] IS NULL OR p.id IN (
               SELECT post_id FROM post_tags WHERE tag_id = ANY($5::uuid[])
             ))
           ORDER BY (COALESCE(pe.likes, 0) + COALESCE(pe.comments, 0) + COALESCE(pe.shares, 0)) DESC NULLS LAST
           LIMIT $6 OFFSET $7"#.to_string()
    } else {
        format!(r#"SELECT id, user_id, integration_id, state as "state: PostState",
                  content, title, media, settings, scheduled_at, published_at,
                  platform_post_id, platform_post_url, error_message,
                  created_at, updated_at,
                  repeat_interval_days, repeat_end_date, group_id,
                  first_comment, sequence, idempotency_key
           FROM posts
           WHERE user_id = $1
             AND deleted_at IS NULL
             AND ($2::text IS NULL OR state = $2::text)
             AND ($3::text IS NULL OR content ILIKE $3 OR title ILIKE $3)
             AND ($4::uuid[] IS NULL OR integration_id = ANY($4::uuid[]))
             AND ($5::uuid[] IS NULL OR id IN (
               SELECT post_id FROM post_tags WHERE tag_id = ANY($5::uuid[])
             ))
           ORDER BY {} LIMIT $6 OFFSET $7"#, order_by)
    };

    let mut q_builder = sqlx::query_as::<_, Post>(&sql)
        .bind(user_id)
        .bind(state_filter)
        .bind(q_pattern);
    // Bind integration_ids as a Vec<Uuid> (sqlx maps this to uuid[])
    q_builder = if let Some(ids) = integration_ids {
        q_builder.bind(ids)
    } else {
        q_builder.bind(None::<&[Uuid]>)
    };
    q_builder = if let Some(ids) = tag_ids {
        q_builder.bind(ids)
    } else {
        q_builder.bind(None::<&[Uuid]>)
    };
    q_builder = q_builder.bind(limit).bind(offset);
    q_builder.fetch_all(pool).await
}

/// Count posts matching the search + filter criteria (for pagination total).
pub async fn count_posts_search(
    pool: &PgPool,
    user_id: Uuid,
    state_filter: Option<&str>,
    q: Option<&str>,
    integration_ids: Option<&[Uuid]>,
    tag_ids: Option<&[Uuid]>,
) -> Result<i64, sqlx::Error> {
    let q_pattern = q.map(|s| {
        let escaped = s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        format!("%{escaped}%")
    });

    let row: (Option<i64>,) = sqlx::query_as(
        r#"SELECT COUNT(*)::bigint FROM posts
           WHERE user_id = $1
             AND ($2::text IS NULL OR state = $2::text)
             AND ($3::text IS NULL OR content ILIKE $3 OR title ILIKE $3)
             AND ($4::uuid[] IS NULL OR integration_id = ANY($4::uuid[]))
             AND ($5::uuid[] IS NULL OR id IN (
               SELECT post_id FROM post_tags WHERE tag_id = ANY($5::uuid[])
             ))"#,
    )
    .bind(user_id)
    .bind(state_filter)
    .bind(q_pattern)
    .bind(integration_ids)
    .bind(tag_ids)
    .fetch_one(pool)
    .await?;
    Ok(row.0.unwrap_or(0))
}

 pub async fn get_post(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Post>, sqlx::Error> {
    // Runtime query (see note in list_posts above).
    sqlx::query_as::<_, Post>(
         r#"SELECT id, user_id, integration_id, state as "state: PostState",
            content, title, media, settings, scheduled_at, published_at,
            platform_post_id, platform_post_url, error_message,
            created_at, updated_at,
            repeat_interval_days, repeat_end_date, group_id,
            first_comment, sequence, idempotency_key
          FROM posts WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"#,
     )
     .bind(id)
     .bind(user_id)
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
    sqlx::query_as::<_, Post>(
        r#"UPDATE posts SET content = $1, title = $2, media = $3, settings = $4,
            updated_at = now()
            WHERE id = $5 AND user_id = $6
            RETURNING id, user_id, integration_id, state as "state: PostState",
               content, title, media, settings, scheduled_at, published_at,
               platform_post_id, platform_post_url, error_message,
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence, idempotency_key"#,
    )
        .bind(content)
        .bind(title)
        .bind(media)
        .bind(settings)
        .bind(id)
        .bind(user_id)
      .fetch_optional(pool)
      .await
  }

  pub async fn schedule_post(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    scheduled_at: DateTime<Utc>,
) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as::<_, Post>(
        r#"UPDATE posts SET scheduled_at = $1, state = 'queued',
            updated_at = now()
            WHERE id = $2 AND user_id = $3
            RETURNING id, user_id, integration_id, state as "state: PostState",
               content, title, media, settings, scheduled_at, published_at,
               platform_post_id, platform_post_url, error_message,
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence, idempotency_key"#,
    )
        .bind(scheduled_at)
        .bind(id)
        .bind(user_id)
      .fetch_optional(pool)
      .await
  }

  /// Phase v21: reset a published post for re-publishing.
  ///
  /// Sets `state = 'queued'`, updates `scheduled_at` to the new time,
  /// and CLEARS `platform_post_id`, `platform_post_url`, `published_at`,
  /// `error_message`, `retry_count`, `next_retry_at`. The scheduler will
  /// then pick it up at the new time and publish a NEW post to the
  /// platform (creating a fresh platform_post_id).
  ///
  /// This is the "Reschedule the post" path of the postiz-inspired
  /// drag-published-post modal.
  ///
  /// Uses a runtime query (not query_as!) because we don't have a live
  /// Postgres to regenerate the .sqlx offline cache with this new SQL.
  pub async fn reset_post_for_republish(
      pool: &PgPool,
      id: Uuid,
      user_id: Uuid,
      scheduled_at: DateTime<Utc>,
  ) -> Result<Option<Post>, sqlx::Error> {
      sqlx::query_as::<_, Post>(
          r#"UPDATE posts SET
              scheduled_at = $1,
              state = 'queued',
              platform_post_id = NULL,
              platform_post_url = NULL,
              published_at = NULL,
              error_message = NULL,
              retry_count = 0,
              next_retry_at = NULL,
              -- Phase v22: generate a NEW idempotency key on re-publish
              -- so the provider treats it as a fresh post (not a retry
              -- of the original publish). Without this, a re-publish
              -- would be deduplicated by the provider and no new post
              -- would be created.
              idempotency_key = gen_random_uuid(),
              updated_at = now()
             WHERE id = $2 AND user_id = $3
             RETURNING id, user_id, integration_id, state as "state: PostState",
               content, title, media, settings, scheduled_at, published_at,
               platform_post_id, platform_post_url, error_message,
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence, idempotency_key"#,
      )
      .bind(scheduled_at)
      .bind(id)
      .bind(user_id)
      .fetch_optional(pool)
      .await
  }

  /// Phase v21: change a post's scheduled_at WITHOUT touching its state
  /// or release fields. Used for the "Just update the post details" path
  /// of the postiz-inspired drag-published-post modal — the user wants
  /// to re-date a published post for archival purposes without triggering
  /// a re-publish.
  ///
  /// Leaves `state`, `platform_post_id`, `platform_post_url`,
  /// `published_at` untouched. Only `scheduled_at` and `updated_at` change.
  pub async fn update_post_date_only(
      pool: &PgPool,
      id: Uuid,
      user_id: Uuid,
      scheduled_at: DateTime<Utc>,
  ) -> Result<Option<Post>, sqlx::Error> {
      sqlx::query_as::<_, Post>(
          r#"UPDATE posts SET scheduled_at = $1, updated_at = now()
             WHERE id = $2 AND user_id = $3
             RETURNING id, user_id, integration_id, state as "state: PostState",
               content, title, media, settings, scheduled_at, published_at,
               platform_post_id, platform_post_url, error_message,
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence, idempotency_key"#,
      )
      .bind(scheduled_at)
      .bind(id)
      .bind(user_id)
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
    // Soft-delete: set deleted_at = NOW() on the post AND on every post
    // sharing the same group_id (if any). This makes the delete reversible
    // and keeps the calendar / posts-list queries clean (they filter
    // `WHERE deleted_at IS NULL`).
    //
    // If the post has no group_id, only the single row is soft-deleted.
    // Runtime query (see note in list_posts above).
    let r = sqlx::query(
        r#"UPDATE posts SET deleted_at = NOW()
           WHERE user_id = $2 AND deleted_at IS NULL AND (
             id = $1
             OR (group_id IS NOT NULL AND group_id = (SELECT group_id FROM posts WHERE id = $1 AND user_id = $2))
           )"#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Hard-undelete a post (and its group). Useful for a future "Trash" UI.
pub async fn undelete_post(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    // Runtime query (see note in list_posts above).
    let r = sqlx::query(
        r#"UPDATE posts SET deleted_at = NULL
           WHERE user_id = $2 AND deleted_at IS NOT NULL AND (
             id = $1
             OR (group_id IS NOT NULL AND group_id = (SELECT group_id FROM posts WHERE id = $1 AND user_id = $2))
           )"#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// List all posts sharing a group_id (for thread/group editing in the composer).
/// Returns empty vec if group_id is None or no posts match.
/// Excludes soft-deleted posts.
pub async fn list_posts_by_group(
    pool: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Vec<Post>, sqlx::Error> {
    // Runtime query (see note in list_posts above).
    sqlx::query_as::<_, Post>(
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
           content, title, media, settings, scheduled_at, published_at,
           platform_post_id, platform_post_url, error_message,
           created_at, updated_at,
           repeat_interval_days, repeat_end_date, group_id,
           first_comment, sequence, idempotency_key
         FROM posts
         WHERE user_id = $1 AND group_id = $2 AND deleted_at IS NULL
         ORDER BY sequence ASC, created_at ASC"#,
    )
    .bind(user_id)
    .bind(group_id)
    .fetch_all(pool)
    .await
}

/// Get a single post with its integration details (used for retry/publish now)
pub async fn get_post_with_integration(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
) -> Result<Option<PostWithIntegration>, sqlx::Error> {
    sqlx::query_as::<_, PostWithIntegration>(
        r#"SELECT p.id, p.user_id, p.integration_id,
             p.state as "state: PostState",
             p.content, p.title, p.media, p.settings,
             p.scheduled_at, p.published_at,
             p.platform_post_id, p.platform_post_url, p.error_message,
             p.created_at, p.updated_at,
             p.repeat_interval_days, p.repeat_end_date, p.group_id,
             p.first_comment, p.sequence, p.idempotency_key,
             i.provider_identifier, i.access_token,
             i.refresh_token, i.token_expires_at,
             i.disabled as "integration_disabled",
             i.refresh_needed as "integration_refresh_needed"
           FROM posts p
           JOIN integrations i ON p.integration_id = i.id
           WHERE p.id = $1 AND p.user_id = $2"#,
    )
        .bind(post_id)
        .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Get posts due for publishing.
///
/// Uses `FOR UPDATE SKIP LOCKED` to claim rows atomically so multiple
/// `social-forge serve` instances running in parallel do not double-publish
/// the same post. Rows are still returned in `state = 'queued'`; the caller
/// is expected to commit/rollback to release the lock.
pub async fn get_due_posts(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PostWithIntegration>, sqlx::Error> {
    // Atomically claim due posts by transitioning them
    // `queued -> publishing` in the same transaction that selects them.
    // This closes the dual-instance double-publish hole: a second
    // `social-forge serve` instance running concurrently will not see
    // rows already claimed by the first, because the WHERE clause
    // filters on `state = 'queued'` and we just flipped them to
    // `'publishing'`.
    //
    // Previously the transaction committed immediately after SELECT,
    // releasing the row lock before publish_post ran -- so two
    // instances could both pull the same queued posts.
    //
    // Uses runtime `sqlx::query_as` (not the `query_as!` macro) so
    // the build doesn't require a live DB or the .sqlx offline cache.
    // The trade-off is no compile-time column type checking — but
    // `PostWithIntegration` derives `FromRow` which handles the
    // runtime deserialization.
    let mut tx = pool.begin().await?;

    let sql = r#"WITH claimed AS (
        UPDATE posts
        SET state = 'publishing',
            updated_at = NOW()
        WHERE id IN (
            SELECT p.id
            FROM posts p
            JOIN integrations i ON p.integration_id = i.id
            WHERE p.state = 'queued'
              AND p.scheduled_at <= NOW()
              AND i.disabled = false
            ORDER BY p.scheduled_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id
    )
    SELECT p.id, p.user_id, p.integration_id,
        p.state,
        p.content, p.title, p.media, p.settings,
        p.scheduled_at, p.published_at,
        p.platform_post_id, p.platform_post_url, p.error_message,
        p.created_at, p.updated_at,
        p.repeat_interval_days, p.repeat_end_date, p.group_id,
        p.first_comment, p.sequence, p.idempotency_key,
        i.provider_identifier, i.access_token,
        i.refresh_token, i.token_expires_at,
        i.disabled as integration_disabled,
        i.refresh_needed as integration_refresh_needed
      FROM posts p
      JOIN integrations i ON p.integration_id = i.id
      JOIN claimed ON p.id = claimed.id"#;

    let rows: Vec<PostWithIntegration> = sqlx::query_as(sql)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(rows)
}

/// Reclaim posts stuck in `publishing` state for longer than the
/// given threshold (e.g. 5 minutes). Called on scheduler startup to
/// recover from a crash that left posts mid-flight. The operator
/// should manually review reclaimed posts before re-queuing, since
/// the platform API may have actually accepted the publish.
pub async fn reclaim_stuck_publishing(
    pool: &PgPool,
    stuck_after_secs: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE posts
         SET state = 'error',
             error_message = CONCAT('Post was stuck in publishing state for > ', $1::text, ' seconds -- manual review required'),
             updated_at = NOW()
         WHERE state = 'publishing'
           AND updated_at < NOW() - make_interval(secs => $1::double precision)",
    )
    .bind(stuck_after_secs as f64)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Record a single publish attempt in the `publish_attempts` audit
/// table. Called by the scheduler on every publish call (success or
/// failure) so the operator has a full history.
pub async fn record_publish_attempt(
    pool: &PgPool,
    post_id: Uuid,
    attempt_number: i32,
    status: &str,
    error_message: Option<&str>,
    started_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO publish_attempts
         (post_id, attempt_number, status, error_message, started_at, finished_at)
         VALUES ($1, $2, $3, $4, $5, NOW())",
    )
    .bind(post_id)
    .bind(attempt_number)
    .bind(status)
    .bind(error_message)
    .bind(started_at)
    .execute(pool)
    .await?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════
// TAGS
// ══════════════════════════════════════════════════════════════

#[derive(sqlx::FromRow)]
pub struct PostTagRow {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_tags_for_post(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<PostTagRow>, sqlx::Error> {
    sqlx::query_as!(
        PostTagRow,
        r#"SELECT t.id, t.name, t.color, t.created_at, t.updated_at
           FROM post_tags pt
           JOIN tags t ON pt.tag_id = t.id
           WHERE pt.post_id = $1 AND t.user_id = $2
           ORDER BY t.name"#,
        post_id,
        user_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn set_post_tags(
    pool: &PgPool,
    post_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Delete existing post_tags for this post
    sqlx::query("DELETE FROM post_tags WHERE post_id = $1")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

    // Insert new post_tags (only if tag_ids is non-empty)
    for &tag_id in tag_ids {
        sqlx::query(
            "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(post_id)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Get posts for a date range (calendar view)
pub async fn get_posts_by_date_range(
    pool: &PgPool,
    user_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as::<_, Post>(
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
            content, title, media, settings, scheduled_at, published_at,
            platform_post_id, platform_post_url, error_message,
            created_at, updated_at,
            repeat_interval_days, repeat_end_date, group_id,
            first_comment, sequence, idempotency_key
          FROM posts
          WHERE user_id = $1
            AND scheduled_at IS NOT NULL
            AND scheduled_at >= $2
            AND scheduled_at <= $3
          ORDER BY scheduled_at ASC"#,
    )
        .bind(user_id)
        .bind(start)
        .bind(end)
    .fetch_all(pool)
    .await
}

/// Get posts for a date range with engagement metrics from analytics_cache
pub async fn get_calendar_posts_with_metrics(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<CalendarPostWithMetrics>, sqlx::Error> {
    sqlx::query_as::<_, CalendarPostWithMetrics>(
        r#"SELECT
           p.id, p.user_id, p.integration_id,
           p.state::text as state,
           p.content, p.title, p.media,
           p.scheduled_at, p.published_at,
           p.platform_post_id, p.platform_post_url,
           p.error_message,
           p.created_at,
           p.repeat_interval_days, p.repeat_end_date,
           p.group_id, p.first_comment, p.sequence,
           i.provider_name as integration_name,
           NULL::bigint as likes,
           NULL::bigint as comments,
           NULL::bigint as shares,
           NULL::bigint as impressions
         FROM posts p
         LEFT JOIN integrations i ON p.integration_id = i.id
         WHERE p.user_id = $1
           AND p.deleted_at IS NULL
           AND (
             -- Queued/draft posts: filter by scheduled_at
             (p.state != 'published' AND p.scheduled_at >= $2 AND p.scheduled_at <= $3)
             OR
             -- Published posts: filter by published_at
             (p.state = 'published' AND p.published_at >= $2 AND p.published_at <= $3)
           )
         ORDER BY
           CASE WHEN p.state = 'published' THEN p.published_at ELSE p.scheduled_at END ASC"#,
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
}

/// Find next free time slot for scheduling
pub async fn find_next_free_slot(
    pool: &PgPool,
    user_id: Uuid,
    integration_id: Option<Uuid>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let now = Utc::now();

    // Get posting_times from integration if provided
    let posting_times: Vec<i64> = if let Some(iid) = integration_id {
        let integration = sqlx::query_scalar!(
            r#"SELECT posting_times FROM integrations WHERE id = $1 AND user_id = $2"#,
            iid,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        integration
            .and_then(|v| v.as_array().cloned())
            .map(|arr| {
                arr.iter()
                    .filter_map(|obj| obj.get("time").and_then(|t| t.as_i64()))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Fallback: if no posting_times configured, use old logic (last + 2h)
    if posting_times.is_empty() {
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
        return Ok(Some(last.unwrap_or(now) + chrono::Duration::hours(2)));
    }

    // Get scheduled posts for the next 14 days
    let end = now + chrono::Duration::days(14);
    let scheduled: Vec<DateTime<Utc>> = if let Some(iid) = integration_id {
        sqlx::query_scalar!(
            r#"SELECT scheduled_at as "scheduled_at!" FROM posts
               WHERE user_id = $1 AND integration_id = $2 AND state != 'error'
               AND scheduled_at >= $3 AND scheduled_at <= $4"#,
            user_id,
            iid,
            now,
            end
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_scalar!(
            r#"SELECT scheduled_at as "scheduled_at!" FROM posts
               WHERE user_id = $1 AND state != 'error'
               AND scheduled_at >= $2 AND scheduled_at <= $3"#,
            user_id,
            now,
            end
        )
        .fetch_all(pool)
        .await?
    };

    // Walk forward day by day, checking each posting_time slot
    let today = now.date_naive();
    for day_offset in 0..14i64 {
        let date = today + chrono::Duration::days(day_offset);
        for &minutes in &posting_times {
            let slot = date
                .and_hms_opt((minutes / 60) as u32, (minutes % 60) as u32, 0)
                .unwrap()
                .and_utc();
            // Skip slots in the past
            if slot <= now {
                continue;
            }
            // Skip slots that already have a post (within 1 minute tolerance)
            let occupied = scheduled.iter().any(|s| (*s - slot).num_minutes().abs() < 1);
            if !occupied {
                return Ok(Some(slot));
            }
        }
    }

    // All slots full in 14 days — fallback
    Ok(Some(now + chrono::Duration::hours(2)))
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
    search: Option<&str>,
) -> Result<Vec<MediaEntry>, sqlx::Error> {
    match search {
        Some(query) => {
            sqlx::query_as!(
                MediaEntry,
                "SELECT id, user_id, original_name, storage_path, mime_type,
                        file_size, width, height, created_at
                 FROM media WHERE user_id = $1 AND original_name ILIKE $4
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3",
                user_id,
                limit,
                offset,
                format!("%{}%", query),
            )
            .fetch_all(pool)
            .await
        }
        None => {
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
    }
}

pub async fn delete_media(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Option<MediaEntry>, sqlx::Error> {
    sqlx::query_as!(
        MediaEntry,
        "DELETE FROM media WHERE id = $1 AND user_id = $2
         RETURNING id, user_id, original_name, storage_path, mime_type,
           file_size, width, height, created_at",
        id,
        user_id,
    )
    .fetch_optional(pool)
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

// ══════════════════════════════════════════════════════════════
// NOTIFICATIONS
// ══════════════════════════════════════════════════════════════

pub async fn create_notification(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    body: &str,
    notification_type: &str,
    reference_type: Option<&str>,
    reference_id: Option<&str>,
) -> Result<Notification, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        r#"INSERT INTO notifications (user_id, title, body, notification_type, reference_type, reference_id)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, user_id, title, body, notification_type, reference_type, reference_id, is_read, created_at"#,
    )
    .bind(user_id)
    .bind(title)
    .bind(body)
    .bind(notification_type)
    .bind(reference_type)
    .bind(reference_id)
    .fetch_one(pool)
    .await
}

pub async fn list_notifications(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Notification>, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        r#"SELECT id, user_id, title, body, notification_type, reference_type, reference_id, is_read, created_at
           FROM notifications WHERE user_id = $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn count_unread_notifications(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: Option<i64> = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM notifications WHERE user_id = $1 AND is_read = false"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.unwrap_or(0))
}

pub async fn mark_notification_read(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Notification>, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        r#"UPDATE notifications SET is_read = true
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, title, body, notification_type, reference_type, reference_id, is_read, created_at"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn mark_all_notifications_read(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        r#"UPDATE notifications SET is_read = true WHERE user_id = $1 AND is_read = false"#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

// ══════════════════════════════════════════════════════════════
// RECURRING POSTS
// ══════════════════════════════════════════════════════════════

pub async fn create_repeated_post(
    pool: &PgPool,
    user_id: Uuid,
    original_id: Uuid,
    scheduled_at: &DateTime<Utc>,
    group_id: Uuid,
) -> Result<Post, sqlx::Error> {
    sqlx::query_as::<_, Post>(
        r#"INSERT INTO posts (user_id, integration_id, title, content, media, settings, scheduled_at, state, repeat_interval_days, repeat_end_date, group_id)
           SELECT p.user_id, p.integration_id, p.title, p.content, p.media, p.settings, $1, p.state, NULL::int4, NULL::timestamptz, $3
           FROM posts p WHERE p.id = $2 AND p.user_id = $4
           RETURNING id, user_id, integration_id, state as "state: PostState",
             content, title, media, settings, scheduled_at, published_at,
             platform_post_id, platform_post_url, error_message,
             created_at, updated_at,
             repeat_interval_days, repeat_end_date, group_id,
             first_comment, sequence, idempotency_key"#,
    )
        .bind(scheduled_at)
        .bind(original_id)
        .bind(group_id)
        .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn set_post_recurring(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    interval_days: i32,
    end_date: &DateTime<Utc>,
    group_id: Uuid,
) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as::<_, Post>(
        r#"UPDATE posts SET repeat_interval_days = $1, repeat_end_date = $2, group_id = $3,
            updated_at = now()
            WHERE id = $4 AND user_id = $5
            RETURNING id, user_id, integration_id, state as "state: PostState",
              content, title, media, settings, scheduled_at, published_at,
              platform_post_id, platform_post_url, error_message,
              created_at, updated_at,
              repeat_interval_days, repeat_end_date, group_id,
              first_comment, sequence, idempotency_key"#,
    )
        .bind(interval_days)
        .bind(end_date)
        .bind(group_id)
        .bind(id)
        .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn set_post_recurring_with_copies(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    interval_days: i32,
    end_date: &DateTime<Utc>,
    group_id: Uuid,
    original_scheduled: &DateTime<Utc>,
) -> Result<(Vec<Uuid>, Vec<String>), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query_as::<_, Post>(
        r#"UPDATE posts SET repeat_interval_days = $1, repeat_end_date = $2, group_id = $3,
           updated_at = now()
           WHERE id = $4 AND user_id = $5
           RETURNING id, user_id, integration_id, state as "state: PostState",
              content, title, media, settings, scheduled_at, published_at,
              platform_post_id, platform_post_url, error_message,
              created_at, updated_at,
              repeat_interval_days, repeat_end_date, group_id,
              first_comment, sequence, idempotency_key"#,
    )
        .bind(interval_days)
        .bind(end_date)
        .bind(group_id)
        .bind(id)
        .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let interval = chrono::Duration::days(interval_days as i64);
    let mut current = *original_scheduled + interval;
    let mut post_ids = Vec::new();
    let mut scheduled_dates = Vec::new();

    while current <= *end_date {
        // Runtime query (Phase v22 — idempotency_key column added).
        let copy = sqlx::query_as::<_, Post>(
            r#"INSERT INTO posts (user_id, integration_id, title, content, media, settings, scheduled_at, state, repeat_interval_days, repeat_end_date, group_id)
               SELECT p.user_id, p.integration_id, p.title, p.content, p.media, p.settings, $1, p.state, NULL::int4, NULL::timestamptz, $3
               FROM posts p WHERE p.id = $2 AND p.user_id = $4
               RETURNING id, user_id, integration_id, state as "state: PostState",
                 content, title, media, settings, scheduled_at, published_at,
                 platform_post_id, platform_post_url, error_message,
                 created_at, updated_at,
                 repeat_interval_days, repeat_end_date, group_id,
                 first_comment, sequence, idempotency_key"#,
        )
        .bind(&current)
        .bind(id)
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        post_ids.push(copy.id);
        scheduled_dates.push(current.to_rfc3339());
        current += interval;
    }

    tx.commit().await?;
    Ok((post_ids, scheduled_dates))
}

// ══════════════════════════════════════════════════════════════
// RSS FEEDS
// ══════════════════════════════════════════════════════════════

pub async fn create_rss_feed(
    pool: &PgPool,
    user_id: Uuid,
    feed_url: &str,
    integration_id: Uuid,
    title: &str,
    use_ai_summary: bool,
    enabled: bool,
) -> Result<RssFeed, sqlx::Error> {
    sqlx::query_as::<_, RssFeed>(
        r#"INSERT INTO rss_feeds (user_id, feed_url, integration_id, title, use_ai_summary, enabled)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, user_id, feed_url, integration_id, title,
             last_polled_at, poll_interval_min, enabled, use_ai_summary,
             created_at, updated_at"#,
    )
    .bind(user_id)
    .bind(feed_url)
    .bind(integration_id)
    .bind(title)
    .bind(use_ai_summary)
    .bind(enabled)
    .fetch_one(pool)
    .await
}

pub async fn list_rss_feeds(pool: &PgPool, user_id: Uuid) -> Result<Vec<RssFeed>, sqlx::Error> {
    sqlx::query_as::<_, RssFeed>(
        r#"SELECT id, user_id, feed_url, integration_id, title,
           last_polled_at, poll_interval_min, enabled, use_ai_summary,
           created_at, updated_at
           FROM rss_feeds WHERE user_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_rss_feed(pool: &PgPool, feed_id: Uuid, user_id: Uuid) -> Result<Option<RssFeed>, sqlx::Error> {
    sqlx::query_as::<_, RssFeed>(
        r#"SELECT id, user_id, feed_url, integration_id, title,
           last_polled_at, poll_interval_min, enabled, use_ai_summary,
           created_at, updated_at
           FROM rss_feeds WHERE id = $1 AND user_id = $2"#,
    )
    .bind(feed_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_rss_feed(pool: &PgPool, feed_id: Uuid, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "DELETE FROM rss_feeds WHERE id = $1 AND user_id = $2",
    )
    .bind(feed_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

pub async fn toggle_rss_feed(pool: &PgPool, feed_id: Uuid, user_id: Uuid) -> Result<Option<RssFeed>, sqlx::Error> {
    sqlx::query_as::<_, RssFeed>(
        r#"UPDATE rss_feeds SET enabled = NOT enabled, updated_at = NOW()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, feed_url, integration_id, title,
             last_polled_at, poll_interval_min, enabled, use_ai_summary,
             created_at, updated_at"#,
    )
    .bind(feed_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_feeds_due_for_polling(pool: &PgPool) -> Result<Vec<RssFeed>, sqlx::Error> {
    sqlx::query_as::<_, RssFeed>(
        "SELECT id, user_id, feed_url, integration_id, title, last_polled_at, poll_interval_min, enabled, use_ai_summary, created_at, updated_at FROM rss_feeds WHERE enabled = true AND (last_polled_at IS NULL OR last_polled_at + (poll_interval_min::text || ' minutes')::interval < NOW())"
    )
    .fetch_all(pool)
    .await
}

pub async fn update_feed_last_polled(pool: &PgPool, feed_id: Uuid) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE rss_feeds SET last_polled_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(feed_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

pub async fn insert_rss_post(
    pool: &PgPool,
    feed_id: Uuid,
    guid: &str,
    title: &str,
    url: &str,
    published_at: Option<DateTime<Utc>>,
    content_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO rss_posts (feed_id, guid, title, url, published_at, content_hash)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (feed_id, guid) DO NOTHING"#,
    )
    .bind(feed_id)
    .bind(guid)
    .bind(title)
    .bind(url)
    .bind(published_at)
    .bind(content_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn check_rss_post_exists(pool: &PgPool, feed_id: Uuid, content_hash: &str) -> Result<bool, sqlx::Error> {
    let count: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM rss_posts WHERE feed_id = $1 AND content_hash = $2",
    )
    .bind(feed_id)
    .bind(content_hash)
    .fetch_one(pool)
    .await?;
    Ok(count.unwrap_or(0) > 0)
}

pub async fn list_rss_feed_items(
    pool: &PgPool,
    feed_id: Uuid,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<RssPost>, sqlx::Error> {
    sqlx::query_as::<_, RssPost>(
        r#"SELECT rp.id, rp.feed_id, rp.post_id, rp.guid, rp.title, rp.url,
           rp.published_at, rp.content_hash, rp.is_imported, rp.created_at
           FROM rss_posts rp
           JOIN rss_feeds rf ON rp.feed_id = rf.id
           WHERE rf.id = $1 AND rf.user_id = $2
           ORDER BY rp.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(feed_id)
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn update_rss_post_post_id(pool: &PgPool, rss_post_id: Uuid, post_id: Uuid) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE rss_posts SET post_id = $1, is_imported = true WHERE id = $2",
    )
    .bind(post_id)
    .bind(rss_post_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

pub async fn get_rss_post_by_hash(pool: &PgPool, feed_id: Uuid, content_hash: &str) -> Result<Option<RssPost>, sqlx::Error> {
    sqlx::query_as::<_, RssPost>(
        r#"SELECT id, feed_id, post_id, guid, title, url, published_at, content_hash, is_imported, created_at
           FROM rss_posts WHERE feed_id = $1 AND content_hash = $2"#,
    )
    .bind(feed_id)
    .bind(content_hash)
    .fetch_optional(pool)
    .await
}

pub async fn delete_notification(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        r#"DELETE FROM notifications WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

// ══════════════════════════════════════════════════════════════
// POST ENGAGEMENT
// ══════════════════════════════════════════════════════════════

/// Upsert post_engagement for an external post.
/// Inserts a new row or updates the existing one on conflict (post_id).
pub async fn upsert_post_engagement(
    pool: &PgPool,
    post_id: Uuid,
    data: &crate::social::EngagementRow,
) -> Result<PostEngagement, sqlx::Error> {
    sqlx::query_as::<_, PostEngagement>(
        r#"INSERT INTO post_engagement
           (post_id, likes, comments, shares, views, saves, quotes, reposts, replies,
            reactions, upvotes, downvotes, upvote_ratio, awards, raw, fetched_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW())
           ON CONFLICT (post_id) DO UPDATE SET
             likes = EXCLUDED.likes,
             comments = EXCLUDED.comments,
             shares = EXCLUDED.shares,
             views = EXCLUDED.views,
             saves = EXCLUDED.saves,
             quotes = EXCLUDED.quotes,
             reposts = EXCLUDED.reposts,
             replies = EXCLUDED.replies,
             reactions = EXCLUDED.reactions,
             upvotes = EXCLUDED.upvotes,
             downvotes = EXCLUDED.downvotes,
             upvote_ratio = EXCLUDED.upvote_ratio,
             awards = EXCLUDED.awards,
             raw = EXCLUDED.raw,
             fetched_at = NOW(),
             updated_at = NOW()
           RETURNING id, post_id, likes, comments, shares, views, saves, quotes, reposts, replies,
             reactions, upvotes, downvotes, upvote_ratio, awards, raw, fetched_at, created_at, updated_at"#,
    )
    .bind(post_id)
    .bind(data.likes)
    .bind(data.comments)
    .bind(data.shares)
    .bind(data.views)
    .bind(data.saves)
    .bind(data.quotes)
    .bind(data.reposts)
    .bind(data.replies)
    .bind(&data.reactions)
    .bind(data.upvotes)
    .bind(data.downvotes)
    .bind(data.upvote_ratio)
    .bind(data.awards)
    .bind(&data.raw)
    .fetch_one(pool)
    .await
}

/// Get engagement data for a specific post.
pub async fn get_post_engagement_by_post_id(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Option<PostEngagement>, sqlx::Error> {
    sqlx::query_as::<_, PostEngagement>(
        r#"SELECT id, post_id, likes, comments, shares, views, saves, quotes, reposts, replies,
           reactions, upvotes, downvotes, upvote_ratio, awards, raw, fetched_at, created_at, updated_at
         FROM post_engagement WHERE post_id = $1"#,
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await
}

/// List external posts with their engagement data LEFT JOINed.
/// Returns posts with engagement_* prefixed fields.
pub async fn list_all_external_posts_with_engagement(
    pool: &PgPool,
    user_id: Uuid,
    provider: Option<&str>,
    author_handle: Option<&str>,
    cursor: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<ExternalPostWithEngagement>, sqlx::Error> {
    if let Some(provider) = provider {
        sqlx::query_as::<_, ExternalPostWithEngagement>(
            r#"SELECT ep.id, ep.user_id, ep.provider, ep.platform_post_id,
               ep.text, ep.author_name, ep.author_handle, ep.author_avatar,
               ep.created_at, ep.url, ep.media, ep.metadata, ep.imported_at,
               pe.likes AS engagement_likes,
               pe.comments AS engagement_comments,
               pe.shares AS engagement_shares,
               pe.views AS engagement_views,
               pe.saves AS engagement_saves,
               pe.quotes AS engagement_quotes,
               pe.reposts AS engagement_reposts,
               pe.replies AS engagement_replies,
               pe.reactions AS engagement_reactions,
               pe.upvotes AS engagement_upvotes,
               pe.downvotes AS engagement_downvotes,
               pe.upvote_ratio AS engagement_upvote_ratio,
               pe.awards AS engagement_awards,
               pe.raw AS engagement_raw,
               pe.fetched_at AS engagement_fetched_at
             FROM external_posts ep
             LEFT JOIN post_engagement pe ON pe.post_id = ep.id
             WHERE ep.user_id = $1 AND ep.provider = $2
               AND ep.hidden_at IS NULL
               AND ($3::timestamptz IS NULL OR ep.created_at < $3)
               AND ($4::text IS NULL OR ep.author_handle = $4)
             ORDER BY ep.created_at DESC
             LIMIT $5"#,
        )
        .bind(user_id)
        .bind(provider)
        .bind(cursor)
        .bind(author_handle)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ExternalPostWithEngagement>(
            r#"SELECT ep.id, ep.user_id, ep.provider, ep.platform_post_id,
               ep.text, ep.author_name, ep.author_handle, ep.author_avatar,
               ep.created_at, ep.url, ep.media, ep.metadata, ep.imported_at,
               pe.likes AS engagement_likes,
               pe.comments AS engagement_comments,
               pe.shares AS engagement_shares,
               pe.views AS engagement_views,
               pe.saves AS engagement_saves,
               pe.quotes AS engagement_quotes,
               pe.reposts AS engagement_reposts,
               pe.replies AS engagement_replies,
               pe.reactions AS engagement_reactions,
               pe.upvotes AS engagement_upvotes,
               pe.downvotes AS engagement_downvotes,
               pe.upvote_ratio AS engagement_upvote_ratio,
               pe.awards AS engagement_awards,
               pe.raw AS engagement_raw,
               pe.fetched_at AS engagement_fetched_at
             FROM external_posts ep
             LEFT JOIN post_engagement pe ON pe.post_id = ep.id
             WHERE ep.user_id = $1
               AND ep.hidden_at IS NULL
               AND ($2::timestamptz IS NULL OR ep.created_at < $2)
               AND ($3::text IS NULL OR ep.author_handle = $3)
             ORDER BY ep.created_at DESC
             LIMIT $4"#,
        )
        .bind(user_id)
        .bind(cursor)
        .bind(author_handle)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Get total engagement summary across all posts for a user (for analytics dashboard).
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct EngagementSummary {
    pub total_likes: Option<i64>,
    pub total_comments: Option<i64>,
    pub total_shares: Option<i64>,
    pub total_views: Option<i64>,
    pub total_reposts: Option<i64>,
    pub total_replies: Option<i64>,
    pub total_upvotes: Option<i64>,
    pub total_awards: Option<i64>,
    pub posts_with_engagement: Option<i64>,
}

pub async fn get_engagement_summary(
    pool: &PgPool,
    user_id: Uuid,
    provider: Option<&str>,
    cutoff: Option<DateTime<Utc>>,
) -> Result<EngagementSummary, sqlx::Error> {
    // Phase 2: add cutoff filter for date-range analytics.
    if let Some(provider) = provider {
        sqlx::query_as::<_, EngagementSummary>(
            r#"SELECT
               SUM(pe.likes)::bigint AS total_likes,
               SUM(pe.comments)::bigint AS total_comments,
               SUM(pe.shares)::bigint AS total_shares,
               SUM(pe.views)::bigint AS total_views,
               SUM(pe.reposts)::bigint AS total_reposts,
               SUM(pe.replies)::bigint AS total_replies,
               SUM(pe.upvotes)::bigint AS total_upvotes,
               SUM(pe.awards)::bigint AS total_awards,
               COUNT(pe.id)::bigint AS posts_with_engagement
             FROM external_posts ep
             INNER JOIN post_engagement pe ON pe.post_id = ep.id
             WHERE ep.user_id = $1 AND ep.provider = $2
               AND ($3::timestamptz IS NULL OR ep.created_at >= $3)"#,
        )
        .bind(user_id)
        .bind(provider)
        .bind(cutoff)
        .fetch_one(pool)
        .await
    } else {
        sqlx::query_as::<_, EngagementSummary>(
            r#"SELECT
               SUM(pe.likes)::bigint AS total_likes,
               SUM(pe.comments)::bigint AS total_comments,
               SUM(pe.shares)::bigint AS total_shares,
               SUM(pe.views)::bigint AS total_views,
               SUM(pe.reposts)::bigint AS total_reposts,
               SUM(pe.replies)::bigint AS total_replies,
               SUM(pe.upvotes)::bigint AS total_upvotes,
               SUM(pe.awards)::bigint AS total_awards,
               COUNT(pe.id)::bigint AS posts_with_engagement
             FROM external_posts ep
             INNER JOIN post_engagement pe ON pe.post_id = ep.id
             WHERE ep.user_id = $1
               AND ($2::timestamptz IS NULL OR ep.created_at >= $2)"#,
        )
        .bind(user_id)
        .bind(cutoff)
        .fetch_one(pool)
        .await
    }
}

// ══════════════════════════════════════════════════════════════
// SIGNATURES
// ══════════════════════════════════════════════════════════════

pub async fn list_signatures(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Signature>, sqlx::Error> {
    // Runtime query (Phase v21/v22 — is_default column added, can't
    // regenerate .sqlx offline cache without a live Postgres).
    sqlx::query_as::<_, Signature>(
        r#"SELECT id, user_id, name, content, provider, is_default,
           created_at, updated_at
           FROM signatures WHERE user_id = $1
           ORDER BY is_default DESC, created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn create_signature(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    content: &str,
    provider: Option<&str>,
) -> Result<Signature, sqlx::Error> {
    sqlx::query_as::<_, Signature>(
        r#"INSERT INTO signatures (user_id, name, content, provider, is_default)
           VALUES ($1, $2, $3, $4, FALSE)
           RETURNING id, user_id, name, content, provider, is_default,
             created_at, updated_at"#,
    )
    .bind(user_id)
    .bind(name)
    .bind(content)
    .bind(provider)
    .fetch_one(pool)
    .await
}

pub async fn update_signature(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
    content: Option<&str>,
    provider: Option<&str>,
) -> Result<Option<Signature>, sqlx::Error> {
    sqlx::query_as::<_, Signature>(
        r#"UPDATE signatures SET
           name = COALESCE($3, name),
           content = COALESCE($4, content),
           provider = COALESCE($5, provider),
           updated_at = now()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, name, content, provider, is_default,
             created_at, updated_at"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(content)
    .bind(provider)
    .fetch_optional(pool)
    .await
}

/// Phase v21/v22: set a signature as the default for its provider.
/// Atomically clears is_default on all other signatures for the same
/// (user_id, provider) and sets it on the target. Uses a transaction
/// so the swap is all-or-nothing.
///
/// `provider` is the target signature's provider (NULL = global).
/// The partial unique index idx_signatures_default_per_provider enforces
/// at-most-one-default per (user_id, provider) at the DB level — this
/// function clears the old default first to avoid a constraint violation.
pub async fn set_default_signature(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Signature>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. Fetch the target signature to get its provider.
    let target: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT provider FROM signatures WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let target = match target {
        Some(t) => t.0,
        None => {
            tx.rollback().await?;
            return Ok(None);
        }
    };

    // 2. Clear is_default on all other signatures with the same (user_id, provider).
    sqlx::query(
        r#"UPDATE signatures SET is_default = FALSE
           WHERE user_id = $1 AND is_default = TRUE
             AND (provider IS NOT DISTINCT FROM $2)"#,
    )
    .bind(user_id)
    .bind(&target)
    .execute(&mut *tx)
    .await?;

    // 3. Set is_default = TRUE on the target.
    let updated = sqlx::query_as::<_, Signature>(
        r#"UPDATE signatures SET is_default = TRUE, updated_at = now()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, name, content, provider, is_default,
             created_at, updated_at"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(updated)
}

/// Phase v21/v22: get the default signature for a given provider.
/// Falls back to the global default (provider IS NULL) if no
/// provider-specific default exists. Returns None if neither exists.
///
/// Used by the composer's auto-append flow: when creating a new post,
/// the frontend calls this to get the signature to append.
pub async fn get_default_signature(
    pool: &PgPool,
    user_id: Uuid,
    provider: Option<&str>,
) -> Result<Option<Signature>, sqlx::Error> {
    // Try provider-specific default first, then global.
    if let Some(p) = provider {
        let provider_specific = sqlx::query_as::<_, Signature>(
            r#"SELECT id, user_id, name, content, provider, is_default,
               created_at, updated_at
               FROM signatures
               WHERE user_id = $1 AND provider = $2 AND is_default = TRUE"#,
        )
        .bind(user_id)
        .bind(p)
        .fetch_optional(pool)
        .await?;
        if provider_specific.is_some() {
            return Ok(provider_specific);
        }
    }
    // Fall back to global default.
    sqlx::query_as::<_, Signature>(
        r#"SELECT id, user_id, name, content, provider, is_default,
           created_at, updated_at
           FROM signatures
           WHERE user_id = $1 AND provider IS NULL AND is_default = TRUE"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_signature(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query!(
        "DELETE FROM signatures WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

// ══════════════════════════════════════════════════════════════
// ANALYTICS CACHE
// ══════════════════════════════════════════════════════════════

/// Get all users (for background cache refresh)
pub async fn list_all_users(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, email, password, name, timezone, created_at, updated_at FROM users"
    )
    .fetch_all(pool)
    .await
}

/// Upsert analytics cache: deletes existing entry for (user_id, provider, platform_post_id)
/// then inserts a fresh one, all in a transaction.
pub async fn upsert_analytics_cache(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    platform_post_id: Option<&str>,
    data: &serde_json::Value,
) -> Result<AnalyticsCache, sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "DELETE FROM analytics_cache WHERE user_id = $1 AND provider = $2 \
         AND (platform_post_id = $3 OR ($3 IS NULL AND platform_post_id IS NULL))",
    )
    .bind(user_id)
    .bind(provider)
    .bind(platform_post_id)
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query_as::<_, AnalyticsCache>(
        "INSERT INTO analytics_cache (user_id, provider, platform_post_id, data) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, user_id, provider, platform_post_id, data, cached_at, expires_at",
    )
    .bind(user_id)
    .bind(provider)
    .bind(platform_post_id)
    .bind(data)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result)
}

/// Get non-expired account-level analytics for (user_id, provider)
pub async fn get_cached_analytics(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    now: DateTime<Utc>,
) -> Result<Vec<AnalyticsCache>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsCache>(
        "SELECT id, user_id, provider, platform_post_id, data, cached_at, expires_at \
         FROM analytics_cache \
         WHERE user_id = $1 AND provider = $2 AND platform_post_id IS NULL AND expires_at > $3 \
         ORDER BY provider",
    )
    .bind(user_id)
    .bind(provider)
    .bind(now)
    .fetch_all(pool)
    .await
}

/// Get a specific cached analytics entry for a post
pub async fn get_single_cached_analytics(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    platform_post_id: &str,
) -> Result<Option<AnalyticsCache>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsCache>(
        "SELECT id, user_id, provider, platform_post_id, data, cached_at, expires_at \
         FROM analytics_cache \
         WHERE user_id = $1 AND provider = $2 AND platform_post_id = $3 AND expires_at > NOW()",
    )
    .bind(user_id)
    .bind(provider)
    .bind(platform_post_id)
    .fetch_optional(pool)
    .await
}

/// Delete all expired analytics cache entries. Returns count of deleted rows.
pub async fn delete_expired_analytics_cache(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query("DELETE FROM analytics_cache WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

// ── External Posts ──────────────────────────────────────────

/// Insert a new external post. Returns the created record.
/// Insert an external post, updating on conflict (provider + platform_post_id).
/// Returns `Some(post)` always — either the newly inserted or the updated record.
pub async fn insert_external_post(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    platform_post_id: &str,
    text: &str,
    author_name: Option<&str>,
    author_handle: Option<&str>,
    author_avatar: Option<&str>,
    created_at: DateTime<Utc>,
    url: Option<&str>,
    media: &serde_json::Value,
    metadata: &serde_json::Value,
) -> Result<Option<ExternalPost>, sqlx::Error> {
    tracing::info!(
        "insert_external_post: provider={} post_id={} text_len={} name={:?} handle={:?} avatar_present={} url_present={}",
        provider, platform_post_id, text.len(), author_name, author_handle, author_avatar.is_some(), url.is_some(),
    );
    let result = sqlx::query_as::<_, ExternalPost>(
        "INSERT INTO external_posts \
         (user_id, provider, platform_post_id, text, author_name, author_handle, author_avatar, created_at, url, media, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (provider, platform_post_id) DO UPDATE SET \
           text = EXCLUDED.text, \
           author_name = COALESCE(EXCLUDED.author_name, external_posts.author_name), \
           author_handle = COALESCE(EXCLUDED.author_handle, external_posts.author_handle), \
           author_avatar = COALESCE(EXCLUDED.author_avatar, external_posts.author_avatar), \
           created_at = EXCLUDED.created_at, \
           url = EXCLUDED.url, \
           media = EXCLUDED.media, \
           metadata = EXCLUDED.metadata, \
           imported_at = now() \
         RETURNING id, user_id, provider, platform_post_id, text,
           author_name, author_handle, author_avatar,
           created_at, url, media, metadata, imported_at",
    )
    .bind(user_id)
    .bind(provider)
    .bind(platform_post_id)
    .bind(text)
    .bind(author_name)
    .bind(author_handle)
    .bind(author_avatar)
    .bind(created_at)
    .bind(url)
    .bind(media)
    .bind(metadata)
    .fetch_optional(pool)
    .await?;
    if let Some(ref post) = result {
        tracing::info!(
            "insert_external_post RETURNED: id={} name={:?} handle={:?} avatar_present={}",
            post.id, post.author_name, post.author_handle, post.author_avatar.is_some(),
        );
    } else {
        tracing::info!("insert_external_post RETURNED: None");
    }
    Ok(result)
}

/// Update the metadata JSON of an external post (used for engagement updates).
pub async fn update_external_post_metadata(
    pool: &PgPool,
    id: Uuid,
    metadata: &serde_json::Value,
) -> Result<ExternalPost, sqlx::Error> {
    sqlx::query_as::<_, ExternalPost>(
        "UPDATE external_posts SET metadata = $1 WHERE id = $2 \
         RETURNING id, user_id, provider, platform_post_id, text,\
           author_name, author_handle, author_avatar,\
           created_at, url, media, metadata, imported_at",
    )
    .bind(metadata)
    .bind(id)
    .fetch_one(pool)
    .await
}

/// Get a single external post by ID and user_id.
pub async fn get_external_post_by_id(
    pool: &PgPool,
    user_id: Uuid,
    post_id: Uuid,
) -> Result<Option<ExternalPost>, sqlx::Error> {
    sqlx::query_as::<_, ExternalPost>(
        "SELECT id, user_id, provider, platform_post_id, text,\
           author_name, author_handle, author_avatar,\
           created_at, url, media, metadata, imported_at \
         FROM external_posts WHERE id = $1 AND user_id = $2",
    )
    .bind(post_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// List external posts for a user + provider, newest first.
pub async fn list_external_posts(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    limit: i64,
) -> Result<Vec<ExternalPost>, sqlx::Error> {
    sqlx::query_as::<_, ExternalPost>(
        "SELECT id, user_id, provider, platform_post_id, text,\
           author_name, author_handle, author_avatar,\
           created_at, url, media, metadata, imported_at \
         FROM external_posts WHERE user_id = $1 AND provider = $2 \
         ORDER BY created_at DESC LIMIT $3",
    )
    .bind(user_id)
    .bind(provider)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// List all external posts for a user across all providers, cursor-paginated by created_at DESC.
/// Pass cursor = None for the first page, then use the last post's created_at as the next cursor.
pub async fn list_all_external_posts(
    pool: &PgPool,
    user_id: Uuid,
    provider: Option<&str>,
    cursor: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<ExternalPost>, sqlx::Error> {
    if let Some(provider) = provider {
        sqlx::query_as::<_, ExternalPost>(
            "SELECT id, user_id, provider, platform_post_id, text,\
               author_name, author_handle, author_avatar,\
               created_at, url, media, metadata, imported_at \
             FROM external_posts \
             WHERE user_id = $1 AND provider = $2 \
               AND ($3::timestamptz IS NULL OR created_at < $3) \
             ORDER BY created_at DESC LIMIT $4",
        )
        .bind(user_id)
        .bind(provider)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ExternalPost>(
            "SELECT id, user_id, provider, platform_post_id, text,\
               author_name, author_handle, author_avatar,\
               created_at, url, media, metadata, imported_at \
             FROM external_posts \
             WHERE user_id = $1 \
               AND ($2::timestamptz IS NULL OR created_at < $2) \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(user_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Search external posts with engagement data LEFT JOINed, mirroring
/// `list_all_external_posts_with_engagement`. Used by the /api/feed?q= endpoint
/// so search results include engagement metrics.
///
/// Case-insensitive ILIKE on `text`, `author_name`, or `author_handle`.
/// `q` is the raw user query — caller should pass it non-empty.
/// LIKE metacharacters (%, _, \) in the query are escaped so user input
/// is treated literally.
pub async fn search_all_external_posts_with_engagement(
    pool: &PgPool,
    user_id: Uuid,
    q: &str,
    provider: Option<&str>,
    cursor: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<ExternalPostWithEngagement>, sqlx::Error> {
    let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("%{escaped}%");

    if let Some(provider) = provider {
        sqlx::query_as::<_, ExternalPostWithEngagement>(
            r#"SELECT ep.id, ep.user_id, ep.provider, ep.platform_post_id,
               ep.text, ep.author_name, ep.author_handle, ep.author_avatar,
               ep.created_at, ep.url, ep.media, ep.metadata, ep.imported_at,
               pe.likes AS engagement_likes,
               pe.comments AS engagement_comments,
               pe.shares AS engagement_shares,
               pe.views AS engagement_views,
               pe.saves AS engagement_saves,
               pe.quotes AS engagement_quotes,
               pe.reposts AS engagement_reposts,
               pe.replies AS engagement_replies,
               pe.reactions AS engagement_reactions,
               pe.upvotes AS engagement_upvotes,
               pe.downvotes AS engagement_downvotes,
               pe.upvote_ratio AS engagement_upvote_ratio,
               pe.awards AS engagement_awards,
               pe.raw AS engagement_raw,
               pe.fetched_at AS engagement_fetched_at
             FROM external_posts ep
             LEFT JOIN post_engagement pe ON pe.post_id = ep.id
             WHERE ep.user_id = $1 AND ep.provider = $2
               AND ($3::timestamptz IS NULL OR ep.created_at < $3)
               AND (ep.text ILIKE $4 OR ep.author_name ILIKE $4 OR ep.author_handle ILIKE $4)
             ORDER BY ep.created_at DESC
             LIMIT $5"#,
        )
        .bind(user_id)
        .bind(provider)
        .bind(cursor)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ExternalPostWithEngagement>(
            r#"SELECT ep.id, ep.user_id, ep.provider, ep.platform_post_id,
               ep.text, ep.author_name, ep.author_handle, ep.author_avatar,
               ep.created_at, ep.url, ep.media, ep.metadata, ep.imported_at,
               pe.likes AS engagement_likes,
               pe.comments AS engagement_comments,
               pe.shares AS engagement_shares,
               pe.views AS engagement_views,
               pe.saves AS engagement_saves,
               pe.quotes AS engagement_quotes,
               pe.reposts AS engagement_reposts,
               pe.replies AS engagement_replies,
               pe.reactions AS engagement_reactions,
               pe.upvotes AS engagement_upvotes,
               pe.downvotes AS engagement_downvotes,
               pe.upvote_ratio AS engagement_upvote_ratio,
               pe.awards AS engagement_awards,
               pe.raw AS engagement_raw,
               pe.fetched_at AS engagement_fetched_at
             FROM external_posts ep
             LEFT JOIN post_engagement pe ON pe.post_id = ep.id
             WHERE ep.user_id = $1
               AND ($2::timestamptz IS NULL OR ep.created_at < $2)
               AND (ep.text ILIKE $3 OR ep.author_name ILIKE $3 OR ep.author_handle ILIKE $3)
             ORDER BY ep.created_at DESC
             LIMIT $4"#,
        )
        .bind(user_id)
        .bind(cursor)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

// ── Resolved comments ──────────────────────────────────────────
// Persists which platform comments the user has marked "resolved"
// in the UI. Comments themselves are fetched live from provider APIs;
// this lightweight table is the only place the resolved flag can live.

/// Mark a comment as resolved for this user. Idempotent — if already
/// resolved, the row is touched (resolved_at refreshed) but no error
/// is returned.
pub async fn resolve_comment(
    pool: &PgPool,
    user_id: Uuid,
    comment_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO resolved_comments (user_id, comment_id, resolved_at) \
         VALUES ($1, $2, NOW()) \
         ON CONFLICT (user_id, comment_id) DO UPDATE SET resolved_at = NOW()",
    )
    .bind(user_id)
    .bind(comment_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Unmark a comment as resolved (re-open it). Idempotent.
pub async fn unresolve_comment(
    pool: &PgPool,
    user_id: Uuid,
    comment_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM resolved_comments WHERE user_id = $1 AND comment_id = $2",
    )
    .bind(user_id)
    .bind(comment_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Return the set of resolved comment IDs for this user.
/// Used by the comments list endpoint to flag each CommentItem.status
/// as "resolved" instead of always "new".
pub async fn list_resolved_comment_ids(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<std::collections::HashSet<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT comment_id FROM resolved_comments WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

// ── Cached comments (B-3) ───────────────────────────────────────
// Cache layer for platform comments. The comments list endpoint reads
// from this table instead of doing 50 sequential provider API calls
// per page load. The background feed refresher writes here.

/// A cached comment row — shape matches the `cached_comments` table.
/// `post_text` is joined in from external_posts so the comments list
/// can show what each comment is replying to without a second query.
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct CachedComment {
    pub id: i64,
    pub user_id: Uuid,
    pub comment_id: String,
    pub post_id: Uuid,
    pub provider: String,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_avatar: Option<String>,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    // Joined from external_posts — the text of the post this comment
    // is replying to. Optional because the post may have been deleted
    // (ON DELETE CASCADE on external_posts.id will remove the cached
    // comment too, but defensively handle NULL here).
    pub post_text: Option<String>,
}

/// Upsert a batch of comments for a single (user_id, post_id).
/// Called by the background feed refresher after it pulls comments
/// from a provider. Existing rows are touched (fetched_at updated)
/// so we know the cache is fresh; new rows are inserted.
///
/// `fetched_at` is set to NOW() for all rows in this batch.
pub async fn upsert_cached_comments(
    pool: &PgPool,
    user_id: Uuid,
    post_id: Uuid,
    provider: &str,
    comments: &[(String, String, Option<String>, Option<String>, Option<String>, DateTime<Utc>)],
) -> Result<(), sqlx::Error> {
    // (comment_id, text, author_name, author_handle, author_avatar, created_at)
    if comments.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (comment_id, text, author_name, author_handle, author_avatar, created_at) in comments {
        sqlx::query(
            r#"INSERT INTO cached_comments
                 (user_id, comment_id, post_id, provider,
                  author_name, author_handle, author_avatar,
                  text, created_at, fetched_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
               ON CONFLICT (user_id, comment_id) DO UPDATE SET
                 post_id = EXCLUDED.post_id,
                 provider = EXCLUDED.provider,
                 author_name = EXCLUDED.author_name,
                 author_handle = EXCLUDED.author_handle,
                 author_avatar = EXCLUDED.author_avatar,
                 text = EXCLUDED.text,
                 created_at = EXCLUDED.created_at,
                 fetched_at = NOW()"#,
        )
        .bind(user_id)
        .bind(comment_id)
        .bind(post_id)
        .bind(provider)
        .bind(author_name)
        .bind(author_handle)
        .bind(author_avatar)
        .bind(text)
        .bind(created_at)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// List cached comments for a user, newest first, with the post text
/// joined in. Optional provider filter. Limited to `limit` rows.
pub async fn list_cached_comments(
    pool: &PgPool,
    user_id: Uuid,
    provider: Option<&str>,
    limit: i64,
) -> Result<Vec<CachedComment>, sqlx::Error> {
    if let Some(provider) = provider {
        sqlx::query_as::<_, CachedComment>(
            r#"SELECT cc.id, cc.user_id, cc.comment_id, cc.post_id, cc.provider,
                      cc.author_name, cc.author_handle, cc.author_avatar,
                      cc.text, cc.created_at, cc.fetched_at,
                      ep.text AS post_text
               FROM cached_comments cc
               LEFT JOIN external_posts ep ON ep.id = cc.post_id
               WHERE cc.user_id = $1 AND cc.provider = $2
               ORDER BY cc.created_at DESC
               LIMIT $3"#,
        )
        .bind(user_id)
        .bind(provider)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, CachedComment>(
            r#"SELECT cc.id, cc.user_id, cc.comment_id, cc.post_id, cc.provider,
                      cc.author_name, cc.author_handle, cc.author_avatar,
                      cc.text, cc.created_at, cc.fetched_at,
                      ep.text AS post_text
               FROM cached_comments cc
               LEFT JOIN external_posts ep ON ep.id = cc.post_id
               WHERE cc.user_id = $1
               ORDER BY cc.created_at DESC
               LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Phase v21: list cached comments for a specific post (used by
/// GET /api/feed/{post_id}/comments). Falls back to live-fetch on cache
/// miss (handled by the caller). Returns newest-first, no limit (the
/// background refresher caps the cache size per post).
pub async fn list_cached_comments_for_post(
    pool: &PgPool,
    user_id: Uuid,
    post_id: Uuid,
) -> Result<Vec<CachedComment>, sqlx::Error> {
    sqlx::query_as::<_, CachedComment>(
        r#"SELECT cc.id, cc.user_id, cc.comment_id, cc.post_id, cc.provider,
                  cc.author_name, cc.author_handle, cc.author_avatar,
                  cc.text, cc.created_at, cc.fetched_at,
                  ep.text AS post_text
           FROM cached_comments cc
           LEFT JOIN external_posts ep ON ep.id = cc.post_id
           WHERE cc.user_id = $1 AND cc.post_id = $2
           ORDER BY cc.created_at ASC"#,
    )
    .bind(user_id)
    .bind(post_id)
    .fetch_all(pool)
    .await
}
