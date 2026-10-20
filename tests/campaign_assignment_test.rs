// ─── A post can be filed under a campaign ─────────────────────
//
// The composer's campaign dropdown set a variable that was NEVER
// READ, so the id was dropped on submit. The kanban's drag PRESERVED
// the existing value rather than setting one. So there was NO UI path
// that could put a post into a campaign:
//
//   - the campaign's `post_count` was always 0
//   - its Posts tab was permanently empty
//   - the kanban's campaign filter showed an empty board
//
// The `campaign_id` column existed and the stage endpoint could write
// it, but the CREATE path could not - and the create path is where a
// post is born.
//
// These tests drive the real router.
//
// Run: cargo test --test campaign_assignment_test
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

async fn seed_integration(state: &AppState) -> Uuid {
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
    integration_id
}

async fn seed_campaign(state: &AppState) -> Uuid {
    let campaign_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO campaigns (id, user_id, name, description, color, status,
             progress_metric, progress_target, created_at, updated_at)
         VALUES (?, ?, ?, 'a campaign', '#6366f1', 'active', 'posts', 10, unixepoch(), unixepoch())",
    )
    .bind(campaign_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("Campaign {}", campaign_id))
    .execute(&state.db)
    .await
    .expect("seed campaign");
    campaign_id
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

/// A post created WITH a campaign is filed under it.
#[tokio::test]
async fn a_post_created_with_a_campaign_is_filed_under_it() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let campaign_id = seed_campaign(&state).await;

    let (status, body) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id.to_string()],
            "content": format!("a post in a campaign {}", Uuid::new_v4()),
            "campaign_id": campaign_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let post_id = json["posts"][0]["id"].as_str().expect("a post id");
    let stored: Option<Uuid> = sqlx::query_scalar("SELECT campaign_id FROM posts WHERE id = ?")
        .bind(Uuid::parse_str(post_id).expect("a uuid"))
        .fetch_one(&state.db)
        .await
        .expect("read campaign_id");
    assert_eq!(
        stored,
        Some(campaign_id),
        "the post must be filed under the campaign: {body}"
    );
}

/// The campaign's post count must reflect the post.
///
/// The count is of the posts that represent WORK: queued, publishing
/// or published. A draft is not work yet - and counting it made a
/// campaign read "10 / 10 - 100%" after the founder created 10 ideas,
/// with nothing written, nothing scheduled and nothing to publish.
///
/// So the test posts a QUEUED post.
#[tokio::test]
async fn the_campaigns_post_count_counts_the_filed_post() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let campaign_id = seed_campaign(&state).await;

    let (status, body) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id.to_string()],
            "content": format!("a post in a campaign {}", Uuid::new_v4()),
            "campaign_id": campaign_id.to_string(),
            "scheduled_at": "2026-12-01T12:00:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call(&state, Method::GET, "/api/campaigns", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let mine = json
        .as_array()
        .expect("an array")
        .iter()
        .find(|c| c["id"] == campaign_id.to_string())
        .expect("the campaign");
    assert_eq!(
        mine["post_count"], 1,
        "the campaign must count the filed post: {body}"
    );
}

/// A DRAFT filed under a campaign is not counted as work.
///
/// The count used to be every post in every state, so a campaign read
/// 100% after the founder had only created ideas.
#[tokio::test]
async fn a_draft_filed_under_a_campaign_is_not_counted_as_work() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let campaign_id = seed_campaign(&state).await;

    let (status, body) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id.to_string()],
            "content": format!("a draft in a campaign {}", Uuid::new_v4()),
            "campaign_id": campaign_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call(&state, Method::GET, "/api/campaigns", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let mine = json
        .as_array()
        .expect("an array")
        .iter()
        .find(|c| c["id"] == campaign_id.to_string())
        .expect("the campaign");
    assert_eq!(
        mine["post_count"], 0,
        "a draft is not work yet, so the campaign must not count it: {body}"
    );
}

/// A post created WITHOUT a campaign is not filed under one.
#[tokio::test]
async fn a_post_created_without_a_campaign_is_not_filed_under_one() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let campaign_id = seed_campaign(&state).await;

    let (status, body) = call(
        &state,
        Method::POST,
        "/api/posts",
        Some(json!({
            "integration_ids": [integration_id.to_string()],
            "content": format!("a post with no campaign {}", Uuid::new_v4()),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let post_id =
        Uuid::parse_str(json["posts"][0]["id"].as_str().expect("a post id")).expect("a uuid");
    let stored: Option<Uuid> = sqlx::query_scalar("SELECT campaign_id FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .expect("read campaign_id");
    assert_eq!(
        stored, None,
        "the post must not be filed under a campaign: {body}"
    );

    // And the campaign is still empty.
    let (status, body) = call(&state, Method::GET, "/api/campaigns", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let mine = json
        .as_array()
        .expect("an array")
        .iter()
        .find(|c| c["id"] == campaign_id.to_string())
        .expect("the campaign");
    assert_eq!(mine["post_count"], 0, "the campaign must be empty: {body}");
}
