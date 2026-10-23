// ─── Editing a post must not delete its provider settings ────
//
// The provider settings hold the TARGET: a Reddit subreddit, a
// Pinterest board, a Discord channel, a Telegram peer, a Hashnode
// publication. The provider reads them at PUBLISH time and errors when
// they are missing.
//
// The composer's edit flow used to delete them. It opened the post,
// the detail response had NO `settings` field to restore from, and the
// composer sent `settings: {}`. The update path REPLACED the settings
// wholesale. So:
//
//   1. the founder schedules a Reddit post for Monday with subreddit
//      "rust" and title "Release 1.0"
//   2. on Monday they open it to fix a typo, edit one word, save
//   3. the subreddit and the title are GONE
//   4. the scheduler picks it up and fails with "Missing subreddit in
//      settings"
//
// The post was working thirty seconds earlier. The same happened to
// every settings-bearing provider.
//
// The fix is two-sided:
//   - the server MERGES the incoming settings over the stored ones, so
//     a key the caller did not send is KEPT
//   - the detail response carries the settings, so the composer can
//     restore and re-send them
//
// These tests drive the real router.
//
// Run: cargo test --test post_settings_preserved_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/switchpost.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
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

/// An integration and a post with the given settings.
async fn seed_post_with_settings(state: &AppState, settings: &Value) -> (Uuid, Uuid) {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'reddit', 'Reddit', ?)",
    )
    .bind(integration_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("reddit-{}", integration_id))
    .execute(&state.db)
    .await
    .expect("seed integration");

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, settings,
                            idempotency_key)
         VALUES (?, ?, ?, 'queued', 'a scheduled reddit post', ?, ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(settings.to_string())
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");
    (integration_id, post_id)
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

/// The stored settings of a post.
async fn stored_settings(state: &AppState, post_id: Uuid) -> Value {
    let raw: String = sqlx::query_scalar("SELECT settings FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .expect("read settings");
    serde_json::from_str(&raw).unwrap_or(Value::Null)
}

/// An edit that does not mention the settings must NOT delete them.
///
/// This is the bug: the composer sent `{}` and the replace wiped the
/// subreddit.
#[tokio::test]
async fn an_edit_that_omits_the_settings_keeps_them() {
    let state = state().await;
    let (_integration_id, post_id) = seed_post_with_settings(
        &state,
        &json!({ "subreddit": "rust", "title": "Release 1.0" }),
    )
    .await;

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({ "content": "a scheduled reddit post, with a typo fixed" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let settings = stored_settings(&state, post_id).await;
    assert_eq!(
        settings["subreddit"], "rust",
        "the subreddit must survive an edit: {settings}"
    );
    assert_eq!(
        settings["title"], "Release 1.0",
        "the title must survive an edit: {settings}"
    );
}

/// An edit that sends an EMPTY settings object must NOT delete them
/// either.
///
/// The composer sent `{}` on every save. With the merge, an empty
/// object adds nothing.
#[tokio::test]
async fn an_edit_with_an_empty_settings_object_keeps_them() {
    let state = state().await;
    let (_integration_id, post_id) = seed_post_with_settings(
        &state,
        &json!({ "subreddit": "rust", "title": "Release 1.0" }),
    )
    .await;

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({
            "content": "a scheduled reddit post, with a typo fixed",
            "settings": {},
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let settings = stored_settings(&state, post_id).await;
    assert_eq!(
        settings["subreddit"], "rust",
        "an empty settings object must not delete the subreddit: {settings}"
    );
}

/// An edit that sends ONE setting sets it and keeps the others.
#[tokio::test]
async fn an_edit_that_sets_one_setting_keeps_the_others() {
    let state = state().await;
    let (_integration_id, post_id) = seed_post_with_settings(
        &state,
        &json!({ "subreddit": "rust", "title": "Release 1.0" }),
    )
    .await;

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({
            "content": "a scheduled reddit post",
            "settings": { "title": "Release 1.1" },
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let settings = stored_settings(&state, post_id).await;
    assert_eq!(
        settings["title"], "Release 1.1",
        "the sent setting must be applied: {settings}"
    );
    assert_eq!(
        settings["subreddit"], "rust",
        "the unsent setting must be kept: {settings}"
    );
}

/// An explicit `null` settings is still a CLEAR attempt, and the
/// server REFUSES it.
///
/// The three-state pattern: absent = keep, `null` = clear, a value =
/// set. The merge must not turn an explicit clear into a no-op - and
/// the required-settings check then refuses the result, because a
/// Reddit post with no subreddit cannot publish.
///
/// The whole UPDATE is refused, so the post keeps its settings and
/// keeps working. The founder is told exactly what is missing, at the
/// moment they did it - not hours later from the scheduler's error
/// log. That is better than accepting the clear and letting the post
/// fail at publish.
#[tokio::test]
async fn an_explicit_null_settings_is_refused_and_the_post_keeps_working() {
    let state = state().await;
    let (_integration_id, post_id) = seed_post_with_settings(
        &state,
        &json!({ "subreddit": "rust", "title": "Release 1.0" }),
    )
    .await;

    let (status, body) = call(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(json!({
            "content": "a scheduled reddit post",
            "settings": Value::Null,
        })),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a Reddit post with no subreddit must be refused: {body}"
    );
    assert!(
        body.contains("subreddit"),
        "the message must name the missing field: {body}"
    );

    // The refused update did NOT change the stored settings - the post
    // is still publishable.
    let settings = stored_settings(&state, post_id).await;
    assert_eq!(
        settings["subreddit"], "rust",
        "the refused update must leave the post publishable: {settings}"
    );
}

/// The detail response carries the settings, so the composer can
/// restore them when it opens a post for editing.
///
/// Without the field, the composer had nothing to send back and every
/// save wiped the target.
#[tokio::test]
async fn the_detail_response_carries_the_settings() {
    let state = state().await;
    let (_integration_id, post_id) = seed_post_with_settings(
        &state,
        &json!({ "subreddit": "rust", "title": "Release 1.0" }),
    )
    .await;

    let (status, body) = call(&state, Method::GET, &format!("/api/posts/{post_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    assert_eq!(
        json["settings"]["subreddit"], "rust",
        "the detail response must carry the settings: {body}"
    );
    assert_eq!(
        json["settings"]["title"], "Release 1.0",
        "the detail response must carry the settings: {body}"
    );
}
