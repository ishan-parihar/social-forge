// ─── API key expiry ────────────────────────────────────────────
// `verify_api_key` accepts a key while `expires_at IS NULL OR expires_at >
// unixepoch()`. The create handler bound `expires_at` as a DateTime, which
// SQLite stores as TEXT. TEXT sorts after every INTEGER, so an expired key
// compared greater than the epoch and kept verifying.
// These tests create keys through the real REST handler and verify them.
//
// Run: cargo test --test api_key_expiry_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::Value;
use social_forge::api::{build_router, AppState};
use social_forge::auth::jwt::create_token;
use social_forge::auth::middleware::{verify_api_key, DEFAULT_USER_ID, SESSION_COOKIE};
use social_forge::config::Config;
use social_forge::db;
use social_forge::realtime::Broadcaster;
use social_forge::social::registry::ProviderRegistry;
use tower::ServiceExt;

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

/// Creates a key through POST /api/developer/api-keys and returns the raw `sf_…` key.
async fn create_key(state: &AppState, expires_at: Option<&str>) -> String {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let body = match expires_at {
        Some(at) => serde_json::json!({ "name": "expiry probe", "expires_at": at }),
        None => serde_json::json!({ "name": "expiry probe" }),
    };
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/developer/api-keys")
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let res = build_router(state.clone())
        .oneshot(req)
        .await
        .expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 16)
        .await
        .expect("body");
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    json["full_key"].as_str().expect("full_key").to_string()
}

#[tokio::test]
async fn key_expired_in_the_past_does_not_verify() {
    let state = state().await;
    let key = create_key(&state, Some("2020-01-01T00:00:00Z")).await;
    assert!(verify_api_key(&state.db, &key).await.is_none());
}

#[tokio::test]
async fn key_expiring_in_the_future_verifies() {
    let state = state().await;
    let key = create_key(&state, Some("2099-01-01T00:00:00Z")).await;
    assert!(verify_api_key(&state.db, &key).await.is_some());
}

#[tokio::test]
async fn key_without_expiry_verifies() {
    let state = state().await;
    let key = create_key(&state, None).await;
    assert!(verify_api_key(&state.db, &key).await.is_some());
}
