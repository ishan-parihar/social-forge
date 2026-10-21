//! The calendar's day bucket follows the caller's UTC offset.
//!
//! The day a post belongs to is a LOCAL day. The day key was
//! `utc.format("%Y-%m-%d")`, so a post at 02:00 UTC on the 25th was on
//! "2026-10-25" - and in New York that instant is 22:00 on the 24th,
//! so the post sat on the WRONG DAY of the calendar. The founder
//! dragged a post to Thursday and it appeared on Friday.
//!
//! The client sends its zone's UTC offset (the server has no timezone
//! database and no per-user zone), and the server buckets the post
//! into the day the founder sees.
//!
//! The test drives the REAL ROUTER, because the behaviour being pinned
//! is the response's day keys.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use social_forge::api::{build_router, AppState};
use social_forge::auth::jwt::create_token;
use social_forge::auth::middleware::{DEFAULT_USER_ID, SESSION_COOKIE};
use social_forge::config::Config;
use social_forge::db;
use social_forge::realtime::Broadcaster;
use social_forge::social::registry::ProviderRegistry;
use tower::ServiceExt;
use uuid::Uuid;

async fn state() -> AppState {
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

async fn seed_integration(state: &AppState) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name,
                                   internal_id, access_token, auth_method, created_at, updated_at)
         VALUES (?, ?, 'x', 'X (Twitter)', ?, 'seed', 'cookie', unixepoch(), unixepoch())",
    )
    .bind(id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("seed-{id}"))
    .execute(&state.db)
    .await
    .expect("seed integration");
    id
}

async fn call(
    state: &AppState,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url);
    let req = match body {
        Some(v) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .expect("request"),
        None => builder.body(Body::empty()).expect("request"),
    };
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

/// The day a given post's content sits on, from the calendar response.
async fn day_of(state: &AppState, query: &str, content: &str) -> String {
    let (status, body) = call(state, Method::GET, query, None).await;
    assert_eq!(status, StatusCode::OK, "calendar failed: {body}");
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let days = v["days"].as_array().expect("days");
    days.iter()
        .find(|d| {
            d["posts"]
                .as_array()
                .is_some_and(|ps| ps.iter().any(|p| p["content"] == *content))
        })
        .map(|d| d["date"].as_str().expect("date").to_string())
        .unwrap_or_else(|| panic!("the post is on no day: {body}"))
}

#[tokio::test]
async fn a_post_at_0200_utc_is_on_the_previous_day_in_new_york() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;

    // 02:00 UTC on the 25th. In New York (UTC-4 in October) that is
    // 22:00 on the 24th.
    let (status, body) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id],
            "content": "a post at 02:00 UTC, bucketed in New York",
            "state": "queued",
            "scheduled_at": "2026-10-25T02:00:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create failed: {body}");

    let date = day_of(
        &state,
        "/api/calendar?start=2026-10-20T00:00:00Z&end=2026-10-30T00:00:00Z&tz_offset=-14400",
        "a post at 02:00 UTC, bucketed in New York",
    )
    .await;
    assert_eq!(
        date, "2026-10-24",
        "02:00 UTC on the 25th is the 24th in New York; the post is on {date}"
    );
}

#[tokio::test]
async fn without_an_offset_the_day_key_is_utc() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;

    let (status, _) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id],
            "content": "a post at 02:00 UTC, bucketed in UTC",
            "state": "queued",
            "scheduled_at": "2026-10-25T02:00:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let date = day_of(
        &state,
        "/api/calendar?start=2026-10-20T00:00:00Z&end=2026-10-30T00:00:00Z",
        "a post at 02:00 UTC, bucketed in UTC",
    )
    .await;
    assert_eq!(
        date, "2026-10-25",
        "with no offset the day key is the UTC day; the post is on {date}"
    );
}

#[tokio::test]
async fn a_post_at_2300_utc_is_on_the_next_day_in_auckland() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;

    // 23:00 UTC on the 24th. Auckland (UTC+13 in October) is 12:00 on
    // the 25th.
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id],
            "content": "a post at 23:00 UTC, bucketed in Auckland",
            "state": "queued",
            "scheduled_at": "2026-10-24T23:00:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let date = day_of(
        &state,
        "/api/calendar?start=2026-10-20T00:00:00Z&end=2026-10-30T00:00:00Z&tz_offset=46800",
        "a post at 23:00 UTC, bucketed in Auckland",
    )
    .await;
    assert_eq!(
        date, "2026-10-25",
        "23:00 UTC on the 24th is the 25th in Auckland; the post is on {date}"
    );
}

#[tokio::test]
async fn an_absurd_offset_is_a_400_not_a_panic() {
    let state = state().await;

    // A POST IN THE WINDOW.
    //
    // The offset is applied per post, when the day key is computed.
    // With no posts in the window the closure never runs, the absurd
    // offset is never added to an instant, and the test proves nothing
    // - it passed with the validation DELETED, because the panic it
    // was waiting for never happened.
    let integration_id = seed_integration(&state).await;
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id],
            "content": "a post, so the offset is actually applied",
            "state": "queued",
            "scheduled_at": "2026-10-25T02:00:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create failed");

    // i64::MAX is 19 digits, so it DESERIALIZES fine and reaches the
    // handler - which is the point. A 20-digit value is refused by the
    // QUERY EXTRACTOR before the handler runs, so a test using one
    // passes on the extractor's rejection and proves nothing about the
    // handler's own validation.
    //
    // `chrono::Duration::seconds` PANICS at magnitudes beyond
    // i64::MAX/1000, and there is no CatchPanic layer in the router -
    // so an unvalidated `tz_offset` took the whole handler down. The
    // handler has to refuse it instead.
    let (status, body) = call(
        &state,
        Method::GET,
        "/api/calendar?start=2026-10-20T00:00:00Z&end=2026-10-30T00:00:00Z&tz_offset=9223372036854775807",
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "an absurd offset must be a 400, not a panic: {body}"
    );
    // The HANDLER's message, not the extractor's. This is what proves
    // the validation branch ran.
    assert!(
        body.contains("tz_offset must be within"),
        "the 400 has to come from the handler's own bound, not the query extractor: {body}"
    );
}
