// ─── UUID id-list filters ──────────────────────────────────────
//
// sqlx encodes a `Uuid` as a 16-byte BLOB, so every uuid column holds
// BLOB bytes. `json_each` yields TEXT, and SQLite never compares a BLOB
// to TEXT as equal — so a filter written as
//
//     id IN (SELECT value FROM json_each(?))
//
// silently matched NOTHING. Every such filter returned an empty list:
// posts filtered by tag, posts filtered by channel, the calendar's
// channel filter, the kanban reorder.
//
// These tests drive the real router and the real query layer, and they
// fail against the old `json_each` expansion.
//
// Run: cargo test --test uuid_filter_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/switchpost.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
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

/// One integration, one post and one tag, all owned by the local user.
async fn seed_post_with_tag(state: &AppState) -> (Uuid, Uuid, Uuid) {
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

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key)
         VALUES (?, ?, ?, 'draft', 'uuid filter probe', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");

    let tag_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tags (id, user_id, name, color) VALUES (?, ?, ?, '#ff0000')")
        .bind(tag_id)
        .bind(DEFAULT_USER_ID)
        .bind(format!("tag-{tag_id}"))
        .execute(&state.db)
        .await
        .expect("seed tag");

    sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES (?, ?)")
        .bind(post_id)
        .bind(tag_id)
        .execute(&state.db)
        .await
        .expect("seed post_tag");

    (integration_id, post_id, tag_id)
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

/// A post is found when the list is filtered by its channel.
#[tokio::test]
async fn filtering_posts_by_channel_returns_the_post() {
    let state = state().await;
    let (integration_id, post_id, _) = seed_post_with_tag(&state).await;

    let (status, body) = get(
        &state,
        &format!("/api/posts?integration_ids={integration_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(&post_id.to_string()),
        "filtering by channel must return the post, got: {body}"
    );
}

/// A post is found when the list is filtered by its tag.
#[tokio::test]
async fn filtering_posts_by_tag_returns_the_post() {
    let state = state().await;
    let (_, post_id, tag_id) = seed_post_with_tag(&state).await;

    let (status, body) = get(&state, &format!("/api/posts?tag_ids={tag_id}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(&post_id.to_string()),
        "filtering by tag must return the post, got: {body}"
    );
}

/// The calendar's channel filter finds the post too.
#[tokio::test]
async fn filtering_the_calendar_by_channel_returns_the_post() {
    let state = state().await;
    let (integration_id, post_id, _) = seed_post_with_tag(&state).await;

    // The post is a draft with no schedule, so put one inside the window.
    // 2027-01-01T12:00:00Z — between the two queried dates.
    sqlx::query("UPDATE posts SET scheduled_at = ? WHERE id = ?")
        .bind(1_798_804_800_i64)
        .bind(post_id)
        .execute(&state.db)
        .await
        .expect("schedule post");

    let (status, body) = get(
        &state,
        &format!("/api/calendar?start=2027-01-01&end=2027-01-02&integration_ids={integration_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(&post_id.to_string()),
        "the calendar channel filter must return the post, got: {body}"
    );
}

/// Assigning a tag to a post is accepted — the ownership check used to
/// reject every real tag because it compared BLOB against TEXT.
#[tokio::test]
async fn assigning_a_tag_to_a_post_is_accepted() {
    let state = state().await;
    let (_, post_id, tag_id) = seed_post_with_tag(&state).await;

    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/posts/{post_id}/tags"))
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(r#"{{"tag_ids":["{tag_id}"]}}"#)))
        .expect("request");
    let res = build_router(state.clone())
        .oneshot(req)
        .await
        .expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 18)
        .await
        .expect("body");
    let body = String::from_utf8_lossy(&bytes).into_owned();

    assert_eq!(
        status,
        StatusCode::OK,
        "a tag the user owns must be assignable, got: {body}"
    );
}

/// The kanban reorder actually moves a card. The UPDATE joined the
/// requested ids against `posts.id` through `json_each`, so it matched
/// no row and reported success while changing nothing.
#[tokio::test]
async fn reordering_the_kanban_persists_the_new_order() {
    let state = state().await;
    let (_, post_id, _) = seed_post_with_tag(&state).await;

    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::PATCH)
        .uri("/api/posts/kanban-reorder")
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"state":"draft","ordered_post_ids":["{post_id}"]}}"#
        )))
        .expect("request");
    let res = build_router(state.clone())
        .oneshot(req)
        .await
        .expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 18)
        .await
        .expect("body");
    let body = String::from_utf8_lossy(&bytes).into_owned();
    assert_eq!(status, StatusCode::OK, "{body}");

    let order: (i64,) = sqlx::query_as("SELECT kanban_sort_order FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .expect("row");
    assert_eq!(order.0, 1, "the reordered post must take slot 1");
}
