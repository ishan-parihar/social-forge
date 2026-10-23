// ─── Analytics engagement for own posts ────────────────────────
//
// GET /api/analytics/engagement reports the engagement of the posts
// the user PUBLISHED. That engagement is stored in `analytics_cache`
// (written by `/api/analytics/post/{id}` after a live provider fetch).
//
// The handler used to read `post_engagement` JOIN `posts`. But
// `post_engagement.post_id` is a foreign key to `external_posts`, so
// that join could never match a row: the endpoint answered 0 for every
// metric, on every database, forever.
//
// These tests drive the real router and read the row back.
//
// Run: cargo test --test analytics_engagement_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/switchpost.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use switchpost::api::{build_router, AppState};
use switchpost::auth::jwt::create_token;
use switchpost::auth::middleware::SESSION_COOKIE;
use switchpost::config::Config;
use switchpost::db;
use switchpost::realtime::Broadcaster;
use switchpost::social::registry::ProviderRegistry;
use tower::ServiceExt;
use uuid::Uuid;

async fn state() -> AppState {
    switchpost::config::load_dotenv_for_tests();
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

/// A fresh user, so each test owns its own posts. The test database is
/// shared between the test binaries in this file, so a shared user would
/// accumulate rows across the tests.
async fn fresh_user(state: &AppState) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Analyst')")
        .bind(id)
        .bind(format!("{id}@analytics.test"))
        .execute(&state.db)
        .await
        .expect("seed user");
    id
}

/// The engagement a provider reported for one post.
#[derive(Debug, Clone, Copy)]
struct Metrics {
    likes: i64,
    comments: i64,
    shares: i64,
    impressions: i64,
}

/// One integration, one published post, and the `analytics_cache` row
/// the provider fetch would have written for it.
async fn seed_published_post_with_analytics(
    state: &AppState,
    user_id: Uuid,
    platform_post_id: &str,
    metrics: Metrics,
    published_at: i64,
) -> Uuid {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'x', 'X (Twitter)', ?)",
    )
    .bind(integration_id)
    .bind(user_id)
    .bind(format!("uid-{integration_id}"))
    .execute(&state.db)
    .await
    .expect("seed integration");

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key,
                            published_at, platform_post_id)
         VALUES (?, ?, ?, 'published', 'own post', ?, ?, ?)",
    )
    .bind(post_id)
    .bind(user_id)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .bind(published_at)
    .bind(platform_post_id)
    .execute(&state.db)
    .await
    .expect("seed post");

    // The provider-shaped payload: a list of {label, data:[{total}]} series.
    let data = format!(
        r#"[{{"label":"Impressions","data":[{{"total":{impressions}}}]}},
            {{"label":"Likes","data":[{{"total":{likes}}}]}},
            {{"label":"Comments","data":[{{"total":{comments}}}]}},
            {{"label":"Shares","data":[{{"total":{shares}}}]}}]"#,
        impressions = metrics.impressions,
        likes = metrics.likes,
        comments = metrics.comments,
        shares = metrics.shares,
    );
    sqlx::query(
        "INSERT INTO analytics_cache (id, user_id, provider, platform_post_id, data, expires_at, cached_at)
         VALUES (?, ?, 'x', ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(platform_post_id)
    .bind(data)
    .bind(i64::MAX)
    .bind(published_at)
    .execute(&state.db)
    .await
    .expect("seed analytics_cache");

    post_id
}

async fn get(state: &AppState, user_id: Uuid, uri: &str) -> (StatusCode, String) {
    let token = create_token(user_id, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .body(Body::empty())
        .expect("request");
    let res = build_router(state.clone())
        .oneshot(req)
        .await
        .expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 18)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// The endpoint reports the engagement of the user's own published posts.
#[tokio::test]
async fn engagement_reports_the_users_own_published_posts() {
    let state = state().await;
    let user_id = fresh_user(&state).await;
    let now = chrono::Utc::now().timestamp();
    seed_published_post_with_analytics(
        &state,
        user_id,
        "qa-own-1",
        Metrics {
            likes: 42,
            comments: 7,
            shares: 9,
            impressions: 1000,
        },
        now - 86_400,
    )
    .await;

    let (status, body) = get(&state, user_id, "/api/analytics/engagement?days=30").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains("\"total_likes\":42"),
        "the published post's 42 likes must be reported, got: {body}"
    );
    assert!(
        body.contains("\"total_comments\":7"),
        "comments must be reported, got: {body}"
    );
    assert!(
        body.contains("\"total_shares\":9"),
        "shares must be reported, got: {body}"
    );
    assert!(
        body.contains("\"total_impressions\":1000"),
        "impressions must be reported, got: {body}"
    );
}

/// A CUMULATIVE series is read at its LAST point, never summed. The
/// provider series (X, YouTube, TikTok) are running totals: a post with
/// 34, 38 and 42 likes has 42 likes, not 114. Every reader used to SUM
/// the points, inflating the same post ~10x on /research and the
/// calendar badge while /analytics showed the true value. This test is
/// the regression pin for that: multi-point series, exact last value.
#[tokio::test]
async fn engagement_reads_the_last_point_of_a_cumulative_series() {
    let state = state().await;
    let user_id = fresh_user(&state).await;
    let now = chrono::Utc::now().timestamp();

    // A multi-point CUMULATIVE series per metric, the shape the live
    // providers emit. The old SUM read 114 likes / 15 comments / 18
    // shares / 2200 impressions; the true current values are below.
    let post_id = Uuid::new_v4();
    let platform_post_id = "qa-cumulative-1";
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'x', 'X', 'qa-cumulative')",
    )
    .bind(integration_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .expect("seed integration");
    let published_at = now - 86_400;
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, platform_post_id,
                            published_at, idempotency_key)
         VALUES (?, ?, ?, 'published', 'cumulative series post', ?, ?, ?)",
    )
    .bind(post_id)
    .bind(user_id)
    .bind(integration_id)
    .bind(platform_post_id)
    .bind(published_at)
    .bind(format!("seed-{post_id}"))
    .execute(&state.db)
    .await
    .expect("seed post");
    let data = r#"[{"label":"Likes","data":[{"total":34},{"total":38},{"total":42}]},
                  {"label":"Comments","data":[{"total":4},{"total":5}]},
                  {"label":"Shares","data":[{"total":6},{"total":9},{"total":18}]}]"#;
    sqlx::query(
        "INSERT INTO analytics_cache (id, user_id, provider, platform_post_id, data, expires_at, cached_at)
         VALUES (?, ?, 'x', ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(platform_post_id)
    .bind(data)
    .bind(i64::MAX)
    .bind(published_at)
    .execute(&state.db)
    .await
    .expect("seed cache");

    let (status, body) = get(&state, user_id, "/api/analytics/engagement?days=30").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains("\"total_likes\":42"),
        "the LAST point of a cumulative Likes series is 42, not the SUM 114: {body}"
    );
    assert!(
        body.contains("\"total_comments\":5"),
        "the LAST point of a cumulative Comments series is 5, not the SUM 9: {body}"
    );
    assert!(
        body.contains("\"total_shares\":18"),
        "the LAST point of a cumulative Shares series is 18, not the SUM 33: {body}"
    );
    assert!(
        !body.contains("\"total_likes\":114"),
        "the SUM of a cumulative series must never surface: {body}"
    );
}

/// The per-day breakdown carries the same numbers, keyed by the day the
/// post was published - the sparkline reads it.
///
/// The series has one point per CALENDAR DAY in the window, including
/// the days with no published post (a zero row). It used to carry only
/// the days that had a publish, so a sparkline connected non-adjacent
/// calendar dates as if they were evenly spaced and a day with zero
/// engagement was not plotted as zero.
#[tokio::test]
async fn engagement_by_day_covers_the_published_post() {
    let state = state().await;
    let user_id = fresh_user(&state).await;
    let now = chrono::Utc::now().timestamp();
    seed_published_post_with_analytics(
        &state,
        user_id,
        "qa-own-2",
        Metrics {
            likes: 11,
            comments: 3,
            shares: 5,
            impressions: 500,
        },
        now - 3_600,
    )
    .await;

    let (status, body) = get(&state, user_id, "/api/analytics/engagement?days=30").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    let by_day = v["by_day"].as_array().expect("by_day");

    // One point per day of the 30-day window.
    assert!(
        by_day.len() >= 28,
        "the series must have a point per calendar day: {body}"
    );

    // The published day carries the post's numbers.
    let published_day = chrono::DateTime::from_timestamp(now - 3_600, 0)
        .expect("ts")
        .format("%Y-%m-%d")
        .to_string();
    let row = by_day
        .iter()
        .find(|r| r["date"] == published_day)
        .expect("the published day must be in the series");
    assert_eq!(row["likes"], 11, "{body}");
    assert_eq!(row["comments"], 3, "{body}");
    assert_eq!(row["shares"], 5, "{body}");
    assert_eq!(row["impressions"], 500, "{body}");

    // A day with no publish is a zero row, not a gap.
    let empty_day = by_day
        .iter()
        .find(|r| r["date"] != published_day)
        .expect("there must be days with no publish");
    assert_eq!(empty_day["likes"], 0, "{body}");
    assert_eq!(empty_day["impressions"], 0, "{body}");
}

/// A post published outside the window is not counted.
#[tokio::test]
async fn engagement_ignores_posts_older_than_the_window() {
    let state = state().await;
    let user_id = fresh_user(&state).await;
    let now = chrono::Utc::now().timestamp();
    seed_published_post_with_analytics(
        &state,
        user_id,
        "qa-own-3",
        Metrics {
            likes: 77,
            comments: 1,
            shares: 1,
            impressions: 900,
        },
        now - 60 * 86_400,
    )
    .await;

    let (status, body) = get(&state, user_id, "/api/analytics/engagement?days=7").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        !body.contains("\"total_likes\":77"),
        "a post older than the window must not be counted, got: {body}"
    );
}
