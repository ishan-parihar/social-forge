// ─── Cookie-auth X channels must not be misdiagnosed by the DMs API ───
//
// X's DM API (`api.twitter.com/2/dm_conversations`) is bearer-only. A
// cookie-auth channel (auth_token + ct0) has no bearer, so the call went
// out with a non-bearer string and X answered 404 - which the classifier
// rendered as "X post or user not found, it may be deleted or
// protected". The founder read a lie about deleted posts when the truth
// is that the DMs surface does not exist for cookie auth at all.
//
// The DMs handlers now refuse cookie-auth X with a message naming the
// real limitation. This test drives the real router.
//
// Run: cargo test --test dms_cookie_auth_test

use axum::http::{Method, StatusCode};
use serde_json::json;
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
    let pool = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    let registry = ProviderRegistry::new(&config, None, None);
    AppState {
        db: pool,
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

/// A cookie-auth X integration, as the channels page creates one.
async fn seed_cookie_x(state: &AppState) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name,
                                   internal_id, access_token, auth_method, created_at, updated_at)
         VALUES (?, ?, 'x', 'X (Twitter)', ?, 'seed-cookie', 'cookie', unixepoch(), unixepoch())",
    )
    .bind(id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("seed-{id}"))
    .execute(&state.db)
    .await
    .expect("seed cookie X integration");
    id
}

async fn call(state: &AppState, method: Method, uri: &str, body: Option<serde_json::Value>) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    let mut req = match body {
        Some(b) => {
            builder = builder.header("content-type", "application/json");
            builder
                .header(axum::http::header::COOKIE, format!("{SESSION_COOKIE}={token}"))
                .body(axum::body::Body::from(serde_json::to_string(&b).unwrap()))
                .unwrap()
        }
        None => builder
            .header(axum::http::header::COOKIE, format!("{SESSION_COOKIE}={token}"))
            .body(axum::body::Body::empty())
            .unwrap(),
    };
    req.headers_mut().insert("origin", state.config.frontend_url.parse().unwrap());
    let res = build_router(state.clone()).oneshot(req).await.expect("call");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// A cookie-auth X channel is refused with the REAL reason - not the
/// deleted-post misdiagnosis the 404 produced.
#[tokio::test]
async fn cookie_auth_x_is_refused_with_the_real_dm_limitation() {
    let state = state().await;
    let integration_id = seed_cookie_x(&state).await;

    let (status, body) = call(
        &state,
        Method::GET,
        &format!("/api/dms/conversations?integration_id={integration_id}&limit=10"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert!(
        body.contains("Cookie-auth channels"),
        "the refusal must name the real limitation: {body}"
    );
    assert!(
        !body.contains("not found"),
        "the deleted-post misdiagnosis must be gone: {body}"
    );
}

/// The send path carries the same refusal - a cookie channel cannot
/// send DMs either.
#[tokio::test]
async fn cookie_auth_x_cannot_send_dms() {
    let state = state().await;
    let integration_id = seed_cookie_x(&state).await;

    let (status, body) = call(
        &state,
        Method::POST,
        "/api/dms/send",
        Some(json!({
            "integration_id": integration_id,
            "recipient": "12345",
            "content": "hello"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert!(
        body.contains("Cookie-auth channels"),
        "the send path refuses with the same reason: {body}"
    );
}
