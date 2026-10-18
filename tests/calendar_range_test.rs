// ─── Calendar range query ──────────────────────────────────────
// `get_calendar_posts_with_metrics` filters by user, deletion and a date range
// that spans two state branches. Its date binds were DateTime (TEXT) against
// INTEGER epoch columns, so no row ever matched, and the OR branch was not
// parenthesised, so it escaped the user and deleted_at filters.
// These tests seed rows directly and read them back through the query.
//
// Run: cargo test --test calendar_range_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use chrono::{DateTime, TimeZone, Utc};
use social_forge::auth::middleware::DEFAULT_USER_ID;
use social_forge::config::Config;
use social_forge::db;
use social_forge::db::queries::get_calendar_posts_with_metrics;
use social_forge::db::SqlitePool;
use uuid::Uuid;

async fn pool() -> SqlitePool {
    social_forge::config::load_dotenv_for_tests();
    let config = Config::from_env().expect("config");
    let pool = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    pool
}

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 11, 0, 0).unwrap()
}

async fn seed_user(pool: &SqlitePool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Other')")
        .bind(id)
        .bind(format!("{id}@calendar.test"))
        .execute(pool)
        .await
        .expect("seed user");
    id
}

async fn seed_integration(pool: &SqlitePool, user_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'bluesky', 'Bluesky', ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(format!("did:plc:{id}"))
    .execute(pool)
    .await
    .expect("seed integration");
    id
}

/// Seeds one post. `scheduled` / `published` are epoch seconds, bound as INTEGER.
async fn seed_post(
    pool: &SqlitePool,
    user_id: Uuid,
    state: &str,
    scheduled: Option<DateTime<Utc>>,
    published: Option<DateTime<Utc>>,
    deleted: bool,
) -> Uuid {
    let integration_id = seed_integration(pool, user_id).await;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key,
                            scheduled_at, published_at, deleted_at)
         VALUES (?, ?, ?, ?, 'calendar probe', ?, ?, ?, CASE WHEN ? THEN unixepoch() END)",
    )
    .bind(id)
    .bind(user_id)
    .bind(integration_id)
    .bind(state)
    .bind(Uuid::new_v4())
    .bind(scheduled.map(|d| d.timestamp()))
    .bind(published.map(|d| d.timestamp()))
    .bind(deleted)
    .execute(pool)
    .await
    .expect("seed post");
    id
}

async fn calendar_ids(pool: &SqlitePool) -> Vec<Uuid> {
    get_calendar_posts_with_metrics(pool, DEFAULT_USER_ID, at(12), at(25), None, None)
        .await
        .expect("calendar query")
        .into_iter()
        .map(|p| p.id)
        .collect()
}

#[tokio::test]
async fn queued_post_inside_range_is_returned() {
    let pool = pool().await;
    let id = seed_post(&pool, DEFAULT_USER_ID, "queued", Some(at(20)), None, false).await;
    assert!(calendar_ids(&pool).await.contains(&id));
}

#[tokio::test]
async fn published_post_inside_range_is_returned() {
    let pool = pool().await;
    let id = seed_post(&pool, DEFAULT_USER_ID, "published", None, Some(at(18)), false).await;
    assert!(calendar_ids(&pool).await.contains(&id));
}

#[tokio::test]
async fn post_outside_range_is_excluded() {
    let pool = pool().await;
    let id = seed_post(&pool, DEFAULT_USER_ID, "queued", Some(at(3)), None, false).await;
    assert!(!calendar_ids(&pool).await.contains(&id));
}

#[tokio::test]
async fn deleted_post_is_excluded() {
    let pool = pool().await;
    let id = seed_post(&pool, DEFAULT_USER_ID, "queued", Some(at(21)), None, true).await;
    assert!(!calendar_ids(&pool).await.contains(&id));
}

#[tokio::test]
async fn other_users_published_post_is_excluded() {
    let pool = pool().await;
    let other = seed_user(&pool).await;
    let id = seed_post(&pool, other, "published", None, Some(at(19)), false).await;
    assert!(!calendar_ids(&pool).await.contains(&id));
}
