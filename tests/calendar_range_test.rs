// ─── Calendar range query ──────────────────────────────────────
// `get_calendar_posts_with_metrics` filters by user, deletion and a date range
// that spans two state branches. Its date binds were DateTime (TEXT) against
// INTEGER epoch columns, so no row ever matched, and the OR branch was not
// parenthesised, so it escaped the user and deleted_at filters.
// These tests seed rows directly and read them back through the query.
//
// Run: cargo test --test calendar_range_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use social_forge::api::{build_router, AppState};
use social_forge::auth::jwt::create_token;
use social_forge::auth::middleware::{DEFAULT_USER_ID, SESSION_COOKIE};
use social_forge::config::Config;
use social_forge::db;
use social_forge::db::queries::get_calendar_posts_with_metrics;
use social_forge::db::SqlitePool;
use social_forge::realtime::Broadcaster;
use social_forge::social::registry::ProviderRegistry;
use tower::ServiceExt;
use uuid::Uuid;

/// A full app state, for the tests that drive the real router.
async fn router_state() -> AppState {
    social_forge::config::load_dotenv_for_tests();
    let config = Config::from_env().expect("config");
    let db = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&db).await.expect("local user");
    let registry = ProviderRegistry::new(&config, None, None);
    AppState {
        db,
        config: config.clone(),
        broadcast: Broadcaster::new(),
        providers: registry,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
        media_http_client: reqwest::Client::new(),
        media_wreq_client: wreq::Client::new(),
    }
}

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

// ─── The end of a calendar range is an INCLUSIVE DATE ──────────
// `end=2026-10-31` was parsed to 2026-10-31T00:00:00Z, and the SQL uses
// `<=`, so every post on the last day of the range was excluded: the 31st
// cell of a month view and the Sunday column of a week view were always
// empty. The end of a date-only range now covers the whole day.

/// A post late on the last day of the range, which the old parser dropped.
async fn seed_last_day_post(state: &AppState) -> Uuid {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'bluesky', 'Bluesky', ?)",
    )
    .bind(integration_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("did:plc:{integration_id}"))
    .execute(&state.db)
    .await
    .expect("seed integration");

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key, scheduled_at)
         VALUES (?, ?, ?, 'queued', 'last day', ?, ?)",
    )
    .bind(id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    // 23:30 on the 31st - the whole point: after midnight.
    .bind(Utc.with_ymd_and_hms(2026, 10, 31, 23, 30, 0).unwrap().timestamp())
    .execute(&state.db)
    .await
    .expect("seed post");
    id
}

#[tokio::test]
async fn the_last_day_of_the_range_is_included() {
    let state = router_state().await;
    let id = seed_last_day_post(&state).await;

    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/calendar?start=2026-10-01&end=2026-10-31")
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .body(Body::empty())
        .expect("request");
    let res = build_router(state.clone()).oneshot(req).await.expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 16).await.expect("body");
    let body: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(status, StatusCode::OK, "{body}");

    let ids: Vec<String> = body["days"]
        .as_array()
        .expect("days")
        .iter()
        .flat_map(|d| d["posts"].as_array().cloned().unwrap_or_default())
        .filter_map(|p| p["id"].as_str().map(|s| s.to_string()))
        .collect();
    assert!(
        ids.contains(&id.to_string()),
        "a post at 23:30 on the end date must be inside the range; got {ids:?}"
    );
}
