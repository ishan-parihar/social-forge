// ─── Editing a post can change its campaign ───────────────────
//
// The `campaign_id` column existed since v22 Phase 6, and the composer
// had a campaign dropdown, but the UPDATE handler never touched the
// column - `update_post_full` wrote content, title, media, settings
// and first_comment, and nothing else.
//
// So filing a post under a campaign on an EDIT silently did nothing.
// The campaign was read-only after creation: the create path wrote it,
// and no path ever changed it.
//
// The fix writes it with the three-state pattern: absent = keep,
// `null` = clear, a value = set.
//
// These tests drive the real router.
//
// Run: cargo test --test campaign_edit_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never
// data/social-forge.db).

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

/// An integration with a unique internal id, so a parallel test run
/// cannot collide on the (provider, internal_id) pair.
async fn seed_integration(state: &AppState) -> Uuid {
    let integration_id = Uuid::new_v4();
    let internal = format!("campaign-edit-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'x', 'X (Twitter)', ?)",
    )
    .bind(integration_id)
    .bind(DEFAULT_USER_ID)
    .bind(&internal)
    .execute(&state.db)
    .await
    .expect("insert integration");
    integration_id
}

async fn seed_campaign(state: &AppState) -> Uuid {
    let campaign_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO campaigns (id, user_id, name, color, start_date, end_date, status)
         VALUES (?, ?, ?, '#6366f1', '2026-01-01', '2026-12-31', 'active')",
    )
    .bind(campaign_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("Campaign {}", Uuid::new_v4()))
    .execute(&state.db)
    .await
    .expect("insert campaign");
    campaign_id
}

async fn seed_post(state: &AppState, integration_id: Uuid) -> Uuid {
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key,
                            created_at, updated_at)
         VALUES (?, ?, ?, 'draft', ?, ?, unixepoch(), unixepoch())",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(format!("campaign edit body {}", Uuid::new_v4()))
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("insert post");
    post_id
}

async fn stored_campaign(state: &AppState, post_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar::<_, Option<Uuid>>("SELECT campaign_id FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_optional(&state.db)
        .await
        .expect("read campaign")
        .flatten()
}

async fn call(
    state: &AppState,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, &state.config.frontend_url);
    let request = match body {
        Some(b) => builder.body(Body::from(b.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = build_router(state.clone())
        .oneshot(request)
        .await
        .expect("router response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap_or_default();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// An edit that sets a campaign FILES the post under it.
#[tokio::test]
async fn an_edit_can_file_the_post_under_a_campaign() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = seed_post(&state, integration_id).await;
    let campaign_id = seed_campaign(&state).await;

    assert_eq!(stored_campaign(&state, post_id).await, None);

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({
            "content": "an edited body",
            "campaign_id": campaign_id.to_string(),
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "the edit must be accepted: {body}");
    assert_eq!(
        stored_campaign(&state, post_id).await,
        Some(campaign_id),
        "the edit must have filed the post under the campaign"
    );
}

/// An edit that does not mention the campaign KEEPS the stored one.
#[tokio::test]
async fn an_edit_that_does_not_mention_the_campaign_keeps_it() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = seed_post(&state, integration_id).await;
    let campaign_id = seed_campaign(&state).await;

    sqlx::query("UPDATE posts SET campaign_id = ? WHERE id = ?")
        .bind(campaign_id)
        .bind(post_id)
        .execute(&state.db)
        .await
        .expect("seed campaign on post");

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({ "content": "an edited body" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "the edit must be accepted: {body}");
    assert_eq!(
        stored_campaign(&state, post_id).await,
        Some(campaign_id),
        "an edit that says nothing about the campaign must keep it"
    );
}

/// An explicit `null` CLEARS the campaign.
#[tokio::test]
async fn an_explicit_null_clears_the_campaign() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = seed_post(&state, integration_id).await;
    let campaign_id = seed_campaign(&state).await;

    sqlx::query("UPDATE posts SET campaign_id = ? WHERE id = ?")
        .bind(campaign_id)
        .bind(post_id)
        .execute(&state.db)
        .await
        .expect("seed campaign on post");

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({ "content": "an edited body", "campaign_id": Value::Null })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "the edit must be accepted: {body}");
    assert_eq!(
        stored_campaign(&state, post_id).await,
        None,
        "an explicit null must clear the campaign"
    );
}
