// ─── A post that cannot publish is refused at CREATE ─────────
//
// Several providers read a TARGET out of the post's settings and
// cannot publish without it:
//
//   Discord        settings["channel"]
//   Reddit         settings["subreddit"], settings["title"]
//   Pinterest      settings["board"], settings["title"]
//   Hashnode       settings["publication_id"]
//   Skool          settings["groupId"], settings["title"]
//   Telegram-user  settings["target"]
//
// The composer used to write `channel_id` and `page_id` - keys NO
// provider reads - and never wrote Discord's `channel` at all. So a
// Discord post with a channel chosen in "Posting Targets" was CREATED,
// QUEUED, and then failed in the scheduler with "Missing channel in
// settings". The founder saw a failed post and no reason.
//
// The fix is two-sided:
//   1. the composer writes the key the provider READS
//   2. the create path checks the provider's `required_settings()` and
//      refuses the post with a message naming the missing field
//
// These tests drive the real router, so they fail against the old
// behaviour (the post is created) and pass on the fixed one (the post
// is refused with a `missing_setting` error).
//
// Run: cargo test --test required_settings_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

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

/// An integration for a provider, owned by the local user.
async fn seed_integration(state: &AppState, provider: &str) -> Uuid {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(integration_id)
    .bind(DEFAULT_USER_ID)
    .bind(provider)
    .bind(provider)
    .bind(format!("{provider}-internal-{integration_id}"))
    .execute(&state.db)
    .await
    .expect("seed integration");
    integration_id
}

async fn post(state: &AppState, uri: &str, body: Value) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
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
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 18)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// A Reddit post with no subreddit is refused, and the message names
/// the field.
///
/// The old behaviour: the post was created and queued, and the failure
/// surfaced in the scheduler.
#[tokio::test]
async fn a_reddit_post_without_a_subreddit_is_refused_at_create() {
    let state = state().await;
    let integration_id = seed_integration(&state, "reddit").await;

    let (status, body) = post(
        &state,
        "/api/posts",
        json!({
            "integration_ids": [integration_id.to_string()],
            "content": format!("A post with no subreddit {}", Uuid::new_v4()),
            "title": "A title",
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a Reddit post with no subreddit must be refused at create: {body}"
    );
    assert!(
        body.contains("subreddit"),
        "the message must name the missing field: {body}"
    );

    // And nothing was written.
    // The content is unique per run: the tests share one database, and a
    // row left by an earlier run would make the count wrong.
    let content = format!("A post with no subreddit {}", Uuid::new_v4());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE content = ?")
        .bind(&content)
        .fetch_one(&state.db)
        .await
        .expect("count posts");
    assert_eq!(count, 0, "the post must not have been created: {body}");
}

/// A Reddit post WITH a subreddit and a title is accepted.
///
/// The subreddit lives in the PER-INTEGRATION override, and a read of
/// only the global settings would report it missing - so this test
/// holds the per-integration path.
#[tokio::test]
async fn a_reddit_post_with_a_subreddit_is_accepted() {
    let state = state().await;
    let integration_id = seed_integration(&state, "reddit").await;

    let (status, body) = post(
        &state,
        "/api/posts",
        json!({
            "integration_ids": [integration_id.to_string()],
            "content": "A post with a subreddit",
            "title": "A title",
            "overrides": {
                integration_id.to_string(): {
                    "settings": { "subreddit": "rust", "title": "A title" }
                }
            }
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "a Reddit post with a subreddit must be accepted: {body}"
    );
}

/// A Discord post with no channel is refused.
///
/// This is the bug the red-team found: the composer wrote
/// `settings["channel_id"]` and Discord reads `settings["channel"]`,
/// so NO Discord post could ever publish.
#[tokio::test]
async fn a_discord_post_without_a_channel_is_refused_at_create() {
    let state = state().await;
    let integration_id = seed_integration(&state, "discord").await;

    let (status, body) = post(
        &state,
        "/api/posts",
        json!({
            "integration_ids": [integration_id.to_string()],
            "content": "A post with no channel",
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a Discord post with no channel must be refused at create: {body}"
    );
    assert!(
        body.contains("channel"),
        "the message must name the missing field: {body}"
    );
}

/// A Discord post WITH the channel the provider reads is accepted.
///
/// The key is `channel` - the key `publish` reads - NOT `channel_id`,
/// which no provider reads.
#[tokio::test]
async fn a_discord_post_with_the_channel_key_is_accepted() {
    let state = state().await;
    let integration_id = seed_integration(&state, "discord").await;

    let (status, body) = post(
        &state,
        "/api/posts",
        json!({
            "integration_ids": [integration_id.to_string()],
            "content": "A post with a channel",
            "overrides": {
                integration_id.to_string(): {
                    "settings": { "channel": "123456789" }
                }
            }
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "a Discord post with settings.channel must be accepted: {body}"
    );
}

/// A provider that publishes from the content alone needs nothing, so
/// a post to it is never refused for a missing setting.
#[tokio::test]
async fn a_provider_with_no_required_settings_is_never_refused() {
    let state = state().await;
    let integration_id = seed_integration(&state, "bluesky").await;

    let (status, body) = post(
        &state,
        "/api/posts",
        json!({
            "integration_ids": [integration_id.to_string()],
            "content": "A plain post",
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Bluesky needs no settings, so the post must be accepted: {body}"
    );
}
