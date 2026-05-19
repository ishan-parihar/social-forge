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
                 first_comment, sequence"#,
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
    sqlx::query_as!(
        Post,
        r#"INSERT INTO posts
           (user_id, integration_id, content, title, media, settings, scheduled_at, state, first_comment, sequence)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, user_id, integration_id, state as "state: PostState",
              content, title, media, settings, scheduled_at, published_at,
              platform_post_id, platform_post_url, error_message,
              created_at, updated_at,
              repeat_interval_days, repeat_end_date, group_id,
              first_comment, sequence"#,
         user_id,
         integration_id,
         content,
         title,
         media,
         settings,
         scheduled_at,
         st as PostState,
         first_comment,
         sequence,
     )
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
) -> Result<Vec<Post>, sqlx::Error> {
    let st = state.unwrap_or(PostState::Draft);
    let mut tx = pool.begin().await?;
    let mut posts = Vec::new();
    let mut seq = 1i32;

    for part in content_parts {
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
                     first_comment, sequence"#,
            )
            .bind(user_id)
            .bind(integration_id)
            .bind(part)
            .bind(None::<&str>)
            .bind(&serde_json::Value::Null)
            .bind(&serde_json::json!({}))
            .bind(scheduled_at)
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
    sqlx::query_as!(
        Post,
        r#"SELECT id, user_id, integration_id, state as "state: PostState",
           content, title, media, settings, scheduled_at, published_at,
           platform_post_id, platform_post_url, error_message,
           created_at, updated_at,
           repeat_interval_days, repeat_end_date, group_id,
           first_comment, sequence
         FROM posts WHERE user_id = $1 AND group_id = $2
         ORDER BY sequence ASC, created_at ASC"#,
        user_id,
        group_id,
    )
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
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence
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
            created_at, updated_at,
            repeat_interval_days, repeat_end_date, group_id,
            first_comment, sequence
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
            created_at, updated_at,
            repeat_interval_days, repeat_end_date, group_id,
            first_comment, sequence
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
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence"#,
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
               created_at, updated_at,
               repeat_interval_days, repeat_end_date, group_id,
               first_comment, sequence"#,
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
             p.repeat_interval_days, p.repeat_end_date, p.group_id,
             p.first_comment, p.sequence,
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
             p.repeat_interval_days, p.repeat_end_date, p.group_id,
             p.first_comment, p.sequence,
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
    sqlx::query_as!(
         Post,
         r#"SELECT id, user_id, integration_id, state as "state: PostState",
            content, title, media, settings, scheduled_at, published_at,
            platform_post_id, platform_post_url, error_message,
            created_at, updated_at,
            repeat_interval_days, repeat_end_date, group_id,
            first_comment, sequence
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
    sqlx::query_as!(
        Post,
        r#"INSERT INTO posts (user_id, integration_id, title, content, media, settings, scheduled_at, state, repeat_interval_days, repeat_end_date, group_id)
           SELECT p.user_id, p.integration_id, p.title, p.content, p.media, p.settings, $1, p.state, NULL::int4, NULL::timestamptz, $3
           FROM posts p WHERE p.id = $2 AND p.user_id = $4
           RETURNING id, user_id, integration_id, state as "state: PostState",
             content, title, media, settings, scheduled_at, published_at,
             platform_post_id, platform_post_url, error_message,
             created_at, updated_at,
             repeat_interval_days, repeat_end_date, group_id,
             first_comment, sequence"#,
        scheduled_at,
        original_id,
        group_id,
        user_id,
    )
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
    sqlx::query_as!(
        Post,
         r#"UPDATE posts SET repeat_interval_days = $1, repeat_end_date = $2, group_id = $3,
            updated_at = now()
            WHERE id = $4 AND user_id = $5
            RETURNING id, user_id, integration_id, state as "state: PostState",
              content, title, media, settings, scheduled_at, published_at,
              platform_post_id, platform_post_url, error_message,
              created_at, updated_at,
              repeat_interval_days, repeat_end_date, group_id,
              first_comment, sequence"#,
        interval_days,
        end_date,
        group_id,
        id,
        user_id,
    )
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

    sqlx::query_as!(
        Post,
        r#"UPDATE posts SET repeat_interval_days = $1, repeat_end_date = $2, group_id = $3,
           updated_at = now()
           WHERE id = $4 AND user_id = $5
           RETURNING id, user_id, integration_id, state as "state: PostState",
              content, title, media, settings, scheduled_at, published_at,
              platform_post_id, platform_post_url, error_message,
              created_at, updated_at,
              repeat_interval_days, repeat_end_date, group_id,
              first_comment, sequence"#,
        interval_days,
        end_date,
        group_id,
        id,
        user_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let interval = chrono::Duration::days(interval_days as i64);
    let mut current = *original_scheduled + interval;
    let mut post_ids = Vec::new();
    let mut scheduled_dates = Vec::new();

    while current <= *end_date {
        let copy = sqlx::query_as!(
            Post,
            r#"INSERT INTO posts (user_id, integration_id, title, content, media, settings, scheduled_at, state, repeat_interval_days, repeat_end_date, group_id)
               SELECT p.user_id, p.integration_id, p.title, p.content, p.media, p.settings, $1, p.state, NULL::int4, NULL::timestamptz, $3
               FROM posts p WHERE p.id = $2 AND p.user_id = $4
               RETURNING id, user_id, integration_id, state as "state: PostState",
                 content, title, media, settings, scheduled_at, published_at,
                 platform_post_id, platform_post_url, error_message,
                 created_at, updated_at,
                 repeat_interval_days, repeat_end_date, group_id,
                 first_comment, sequence"#,
            &current,
            id,
            group_id,
            user_id,
        )
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
// SIGNATURES
// ══════════════════════════════════════════════════════════════

pub async fn list_signatures(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Signature>, sqlx::Error> {
    sqlx::query_as!(
        Signature,
        r#"SELECT id, user_id, name, content, provider,
           created_at, updated_at
           FROM signatures WHERE user_id = $1
           ORDER BY created_at DESC"#,
        user_id,
    )
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
    sqlx::query_as!(
        Signature,
        r#"INSERT INTO signatures (user_id, name, content, provider)
           VALUES ($1, $2, $3, $4)
           RETURNING id, user_id, name, content, provider,
             created_at, updated_at"#,
        user_id,
        name,
        content,
        provider,
    )
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
    sqlx::query_as!(
        Signature,
        r#"UPDATE signatures SET
           name = COALESCE($3, name),
           content = COALESCE($4, content),
           provider = COALESCE($5, provider),
           updated_at = now()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, name, content, provider,
             created_at, updated_at"#,
        id,
        user_id,
        name,
        content,
        provider,
    )
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
