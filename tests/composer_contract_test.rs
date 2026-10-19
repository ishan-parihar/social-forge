// ─── Composer contract: publish, clear, sanitize ───────────────
//
// Three defects in the composer's REST contract, each of which the
// composer exercised on every save:
//
//   1. "Post Now" could never publish. The composer created a post with
//      no schedule (which the backend means by `draft`) and then called
//      `POST /api/posts/{id}/publish`; the publish service refused
//      drafts with "Draft posts must be scheduled first". The composer's
//      primary button was 100% broken.
//
//   2. Clearing a field did not clear it. The update request is a
//      partial update, so an OMITTED field keeps its stored value - but
//      the composer sent `title || undefined`, and `undefined` means
//      "omitted" on the wire. Emptying the title box left the old title
//      on the post. The fix is a three-state field: absent keeps, null
//      clears, a value sets.
//
//   3. The rich-text editor produced TipTap HTML and the REST create
//      path stored it verbatim, so a post with bold text shipped
//      `<p>Hello <strong>world</strong></p>` to the platform as the
//      tweet text.
//
// These tests drive the real router and read the row back.
//
// Run: cargo test --test composer_contract_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::Value;
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
async fn seed_integration(state: &AppState) -> Uuid {
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
    integration_id
}

async fn request(
    state: &AppState,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url);
    let body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let req = builder.body(body).expect("request");
    let res = build_router(state.clone()).oneshot(req).await.expect("router");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 16).await.expect("body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn post_row(state: &AppState, id: Uuid) -> (String, Option<String>, Option<String>, String) {
    sqlx::query_as::<_, (String, Option<String>, Option<String>, String)>(
        "SELECT state, title, first_comment, content FROM posts WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .expect("row")
}

// ─── 1. A draft can be published ───────────────────────────────

#[tokio::test]
async fn a_draft_is_claimable_for_publish() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key)
         VALUES (?, ?, ?, 'draft', 'publish probe', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");

    // The claim is the gate: the publish service moves the post to
    // `publishing` before it talks to the provider. It used to accept
    // only `queued` and `error`, so a draft could never be published
    // and the composer's "Post Now" button was dead.
    let claimed = db::queries::claim_post_for_publish(&state.db, post_id, DEFAULT_USER_ID)
        .await
        .expect("claim");
    assert!(
        claimed,
        "a draft must be claimable for publish - the composer's Post Now creates one"
    );
    let (state_name, _, _, _) = post_row(&state, post_id).await;
    assert_eq!(state_name, "publishing");
}

#[tokio::test]
async fn the_scheduler_still_cannot_claim_a_draft() {
    // The scheduler's claim is a DIFFERENT query and must stay
    // `queued`-only: a draft the founder saved must never publish on
    // its own.
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key, scheduled_at)
         VALUES (?, ?, ?, 'draft', 'scheduler probe', ?, ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .bind(chrono::Utc::now().timestamp() - 3600)
    .execute(&state.db)
    .await
    .expect("seed post");

    let due = db::queries::get_due_posts(&state.db, 10)
        .await
        .expect("claim due");
    assert!(
        !due.iter().any(|p| p.id == post_id),
        "the scheduler must not pick up a draft"
    );
}

// ─── 2. Clearing a field clears it ────────────────────────────

#[tokio::test]
async fn an_explicit_null_title_clears_the_title() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, title, idempotency_key)
         VALUES (?, ?, ?, 'draft', 'body', 'the old title', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");

    let (status, body) = request(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(serde_json::json!({ "content": "body", "title": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, title, _, _) = post_row(&state, post_id).await;
    assert_eq!(title, None, "an explicit null must clear the title");
}

#[tokio::test]
async fn an_omitted_title_keeps_the_title() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, title, idempotency_key)
         VALUES (?, ?, ?, 'draft', 'body', 'the old title', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");

    let (status, body) = request(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(serde_json::json!({ "content": "body" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, title, _, _) = post_row(&state, post_id).await;
    assert_eq!(
        title.as_deref(),
        Some("the old title"),
        "an omitted field must keep its stored value"
    );
}

#[tokio::test]
async fn an_explicit_null_first_comment_clears_it() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, first_comment, idempotency_key)
         VALUES (?, ?, ?, 'draft', 'body', 'old comment', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");

    let (status, body) = request(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(serde_json::json!({ "content": "body", "first_comment": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, _, first_comment, _) = post_row(&state, post_id).await;
    assert_eq!(first_comment, None, "an explicit null must clear it");
}

// ─── 3. Rich text is sanitized on the way in ───────────────────

#[tokio::test]
async fn create_strips_html_from_the_body() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;

    let (status, body) = request(
        &state,
        Method::POST,
        "/api/posts",
        Some(serde_json::json!({
            "integration_ids": [integration_id.to_string()],
            "content": "<p>Hello <strong>world</strong></p>",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let id = body["posts"][0]["id"]
        .as_str()
        .expect("post id")
        .parse::<Uuid>()
        .expect("uuid");
    let (_, _, _, content) = post_row(&state, id).await;
    assert!(
        !content.contains('<'),
        "the stored body must not carry HTML markup, got {content:?}"
    );
    assert!(content.contains("Hello"), "the text must survive: {content:?}");
}

#[tokio::test]
async fn update_strips_html_from_the_body() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key)
         VALUES (?, ?, ?, 'draft', 'plain', ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");

    let (status, body) = request(
        &state,
        Method::PUT,
        &format!("/api/posts/{post_id}"),
        Some(serde_json::json!({ "content": "<p>Hello <em>there</em></p>" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, _, _, content) = post_row(&state, post_id).await;
    assert!(
        !content.contains('<'),
        "the stored body must not carry HTML markup, got {content:?}"
    );
}
