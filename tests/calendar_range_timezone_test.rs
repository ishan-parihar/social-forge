// ─── The calendar's range filter and its grid must agree ──────
//
// The calendar's GRID is built from the APP's timezone days - the
// events are placed by `wallClock(iso, timezone)`.
//
// The RANGE FILTER was a different definition of "a day". The client
// sent date-only strings (`start=2026-10-01&end=2026-10-31`) and the
// server resolved them to UTC midnight and UTC end-of-day. So the
// filter window was [Oct 1 00:00 UTC, Oct 31 23:59:59 UTC] while the
// grid's cells were the app's timezone days.
//
// The posts in the gap FELL OFF THE CALENDAR. With the app in
// America/New_York, a post scheduled Oct 31 20:00 EDT = Nov 1 00:00
// UTC:
//
//   - October's month view sends start=2026-10-01&end=2026-10-31; the
//     server resolves `end` to Oct 31 23:59:59 UTC; the post (Nov 1
//     00:00 UTC) is EXCLUDED
//   - November's month view sends start=2026-11-01; the post IS
//     fetched, but `wallClock` maps it to 2026-10-31, and November's
//     grid contains only Nov 1-30 plus December trailing days - there
//     is no Oct 31 cell. The post is dropped on the floor.
//
// The post appeared on NO month grid, in either month.
//
// The fix is on the client: the range is sent as UTC INSTANTS computed
// with `wallClockToUtc` - the ONE conversion the app has - so the
// server's exact-datetime path is used and the grid and the filter
// agree by construction.
//
// These tests hold the SERVER's half: a range given as UTC instants
// includes exactly the posts in that window, and the inclusive end is
// the last instant of the day the client computed.
//
// Run: cargo test --test calendar_range_timezone_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/switchpost.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::Value;
use switchpost::api::{build_router, AppState};
use switchpost::auth::jwt::create_token;
use switchpost::auth::middleware::{DEFAULT_USER_ID, SESSION_COOKIE};
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

/// A post scheduled at a given UTC instant.
async fn seed_post_at(state: &AppState, scheduled_at: chrono::DateTime<chrono::Utc>) -> Uuid {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'x', 'X (Twitter)', ?)",
    )
    .bind(integration_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("x-{}", integration_id))
    .execute(&state.db)
    .await
    .expect("seed integration");

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, scheduled_at,
                            idempotency_key)
         VALUES (?, ?, ?, 'queued', 'a scheduled post', ?, ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(scheduled_at.timestamp())
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");
    post_id
}

async fn get(state: &AppState, uri: &str) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
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

/// The post ids in a calendar range.
async fn posts_in_range(state: &AppState, start: &str, end: &str) -> Vec<String> {
    let uri = format!("/api/calendar?start={}&end={}", start, end);
    let (status, body) = get(state, &uri).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let mut ids: Vec<String> = Vec::new();
    for day in json["days"].as_array().expect("a days array") {
        for p in day["posts"].as_array().expect("a posts array") {
            ids.push(p["id"].as_str().expect("a post id").to_string());
        }
    }
    ids
}

/// A post at the very END of a range is included.
///
/// The end is the last instant of the end day. A post one second
/// later is excluded.
#[tokio::test]
async fn the_end_of_the_range_is_inclusive() {
    let state = state().await;
    // 2026-10-31 23:59:59 UTC
    let inside = seed_post_at(
        &state,
        chrono::DateTime::parse_from_rfc3339("2026-10-31T23:59:59Z")
            .expect("a timestamp")
            .with_timezone(&chrono::Utc),
    )
    .await;
    // 2026-11-01 00:00:01 UTC - one second past the end
    let outside = seed_post_at(
        &state,
        chrono::DateTime::parse_from_rfc3339("2026-11-01T00:00:01Z")
            .expect("a timestamp")
            .with_timezone(&chrono::Utc),
    )
    .await;

    let ids = posts_in_range(
        &state,
        "2026-10-01T00:00:00.000Z",
        "2026-10-31T23:59:59.000Z",
    )
    .await;

    assert!(
        ids.contains(&inside.to_string()),
        "the last instant of the end day must be included: {ids:?}"
    );
    assert!(
        !ids.contains(&outside.to_string()),
        "a post one second past the end must be excluded: {ids:?}"
    );
}

/// A post at the very START of a range is included.
#[tokio::test]
async fn the_start_of_the_range_is_inclusive() {
    let state = state().await;
    let inside = seed_post_at(
        &state,
        chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
            .expect("a timestamp")
            .with_timezone(&chrono::Utc),
    )
    .await;
    let outside = seed_post_at(
        &state,
        chrono::DateTime::parse_from_rfc3339("2026-09-30T23:59:59Z")
            .expect("a timestamp")
            .with_timezone(&chrono::Utc),
    )
    .await;

    let ids = posts_in_range(
        &state,
        "2026-10-01T00:00:00.000Z",
        "2026-10-31T23:59:59.000Z",
    )
    .await;

    assert!(
        ids.contains(&inside.to_string()),
        "the first instant of the start day must be included: {ids:?}"
    );
    assert!(
        !ids.contains(&outside.to_string()),
        "a post one second before the start must be excluded: {ids:?}"
    );
}

/// A date-only range still works, and covers the WHOLE UTC day.
///
/// The client now sends instants, but a date-only range is still a
/// valid request - and it must keep meaning "that whole UTC day", the
/// way it always has.
#[tokio::test]
async fn a_date_only_range_covers_the_whole_utc_day() {
    let state = state().await;
    let inside = seed_post_at(
        &state,
        chrono::DateTime::parse_from_rfc3339("2026-10-01T12:00:00Z")
            .expect("a timestamp")
            .with_timezone(&chrono::Utc),
    )
    .await;

    let ids = posts_in_range(&state, "2026-10-01", "2026-10-01").await;
    assert!(
        ids.contains(&inside.to_string()),
        "a date-only range must cover the whole day: {ids:?}"
    );
}
