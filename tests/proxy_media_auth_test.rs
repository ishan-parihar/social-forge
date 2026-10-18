// ─── /api/proxy-media requires the session ─────────────────────
// The route fetches arbitrary allowlisted CDN URLs on the caller's behalf, so
// it sits behind auth_middleware with the other protected routes. Without the
// fix it was public: an anonymous caller reached the upstream fetch.
//
// Run: cargo test --test proxy_media_auth_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use social_forge::api::{build_router, AppState};
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

#[tokio::test]
async fn proxy_media_without_session_is_rejected_before_fetch() {
    let state = state().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/proxy-media?url=https%3A%2F%2Fpbs.twimg.com%2Fmedia%2Fx.jpg")
        .body(Body::empty())
        .expect("request");
    let res = build_router(state).oneshot(req).await.expect("router");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
