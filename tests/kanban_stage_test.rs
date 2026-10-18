// ─── Kanban stage transitions over REST ────────────────────────
// PATCH /api/posts/{id}/stage moves a post between kanban columns. Its UPDATE
// binds were written in the wrong order for SQLite's positional `?`, so the
// WHERE clause matched no row and the handler still answered `updated: true`.
// These tests drive the real router and read the row back.
//
// Run: cargo test --test kanban_stage_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
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

/// One integration and one draft post owned by the local user.
async fn seed_draft(state: &AppState) -> Uuid {
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
         VALUES (?, ?, ?, 'draft', 'kanban probe', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");
    post_id
}

async fn patch_stage(state: &AppState, post_id: Uuid, body: &str) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/posts/{post_id}/stage"))
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let res = build_router(state.clone()).oneshot(req).await.expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 16).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn row(state: &AppState, post_id: Uuid) -> (String, Option<String>, Option<i64>) {
    sqlx::query_as(
        "SELECT state, priority, due_date FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_one(&state.db)
    .await
    .expect("row")
}

#[tokio::test]
async fn moving_a_draft_to_scheduled_persists() {
    let state = state().await;
    let post_id = seed_draft(&state).await;

    let (status, body) = patch_stage(&state, post_id, r#"{"state":"queued"}"#).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(row(&state, post_id).await.0, "queued", "state must persist, not just report success");
}

#[tokio::test]
async fn same_column_update_persists_priority_and_due_date() {
    let state = state().await;
    let post_id = seed_draft(&state).await;

    let (status, body) = patch_stage(
        &state,
        post_id,
        r#"{"state":"draft","priority":"high","due_date":"2030-01-01T00:00:00Z"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (stage, priority, due) = row(&state, post_id).await;
    assert_eq!(stage, "draft");
    assert_eq!(priority.as_deref(), Some("high"));
    assert_eq!(due, Some(1_893_456_000), "due_date must be the RFC3339 epoch");
}

#[tokio::test]
async fn illegal_transition_into_published_is_rejected() {
    let state = state().await;
    let post_id = seed_draft(&state).await;

    let (status, _) = patch_stage(&state, post_id, r#"{"state":"published"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(row(&state, post_id).await.0, "draft");
}
