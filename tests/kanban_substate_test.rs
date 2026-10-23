// ─── A kanban substate can be cleared ─────────────────────────
//
// The kanban card's substate is one of `ready_to_publish`,
// `in_review` or `blocked`, and the filter bar offers a "No substate"
// option.
//
// The field was a plain `Option<String>` and the UPDATE was
// `kanban_substate = COALESCE(?, kanban_substate)` - which cannot
// express "clear", only "keep" and "set". So:
//
//   - the client REFUSED the clear and toasted "Clearing substate is
//     not yet supported - set a different one instead"
//   - a card tagged "Blocked" was "Blocked" for ever
//   - the "No substate" filter could never match a card the founder
//     wanted to clear
//
// The field is three-state now: absent = keep, `null` = clear, a value
// = set. The SQL is a CASE with an explicit flag, because COALESCE
// cannot tell "keep" from "clear" either.
//
// These tests drive the real router.
//
// Run: cargo test --test kanban_substate_test
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

/// A draft post owned by the local user.
async fn seed_post(state: &AppState, substate: Option<&str>) -> Uuid {
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

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, kanban_substate,
                            idempotency_key)
         VALUES (?, ?, ?, 'draft', 'a card', ?, ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(substate)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");
    post_id
}

async fn patch(state: &AppState, post_id: Uuid, body: Value) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/posts/{post_id}/stage"))
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

async fn substate_of(state: &AppState, post_id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT kanban_substate FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .expect("read substate")
}

/// A substate can be SET.
#[tokio::test]
async fn a_substate_can_be_set() {
    let state = state().await;
    let post_id = seed_post(&state, None).await;

    let (status, body) = patch(
        &state,
        post_id,
        json!({ "state": "draft", "kanban_substate": "blocked" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        substate_of(&state, post_id).await.as_deref(),
        Some("blocked")
    );
}

/// A substate can be CLEARED - the "No substate" state the filter bar
/// offers.
///
/// This is the case that could not happen before: the field was a
/// plain `Option<String>` and the SQL was a COALESCE, so the client
/// had to refuse the clear with "Clearing substate is not yet
/// supported".
#[tokio::test]
async fn a_substate_can_be_cleared() {
    let state = state().await;
    let post_id = seed_post(&state, Some("blocked")).await;

    let (status, body) = patch(
        &state,
        post_id,
        json!({ "state": "draft", "kanban_substate": Value::Null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        substate_of(&state, post_id).await,
        None,
        "the substate must be cleared: {body}"
    );
}

/// A request that does not mention the substate LEAVES it alone.
///
/// The third state. A caller that does not mention the field must not
/// clear it by accident.
#[tokio::test]
async fn a_request_that_omits_the_substate_leaves_it_alone() {
    let state = state().await;
    let post_id = seed_post(&state, Some("in_review")).await;

    let (status, body) = patch(&state, post_id, json!({ "state": "draft" })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        substate_of(&state, post_id).await.as_deref(),
        Some("in_review"),
        "the omitted substate must be kept: {body}"
    );
}

/// An invalid substate is refused with a friendly message.
#[tokio::test]
async fn an_invalid_substate_is_refused() {
    let state = state().await;
    let post_id = seed_post(&state, None).await;

    let (status, body) = patch(
        &state,
        post_id,
        json!({ "state": "draft", "kanban_substate": "nonsense" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body.contains("kanban_substate"),
        "the message must name the field: {body}"
    );
}

// ─── A reorder of a mixed column renumbers every card in it ───
//
// The board's Drafts column holds the drafts AND the error posts. The
// reorder's UPDATE was guarded by `AND state = ?` with the column's
// own name, so the error posts in the ordered list did not match, were
// silently not renumbered, and kept their old order.
//
// So a founder who dragged a failed post to the top of Drafts saw it
// move locally, and on the next reload it jumped back. The ordering
// was silently lost, and the response reported "updated" anyway.

/// A post in a given state, owned by the local user.
async fn seed_post_in_state(state: &AppState, post_state: &str) -> Uuid {
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

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, kanban_sort_order,
                            idempotency_key)
         VALUES (?, ?, ?, ?, 'a card', 0, ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(post_state)
    .bind(Uuid::new_v4())
    .execute(&state.db)
    .await
    .expect("seed post");
    post_id
}

async fn sort_order_of(state: &AppState, post_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT kanban_sort_order FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .expect("read sort order")
}

async fn reorder(
    state: &AppState,
    column: &str,
    ids: &[Uuid],
    column_states: Option<Vec<&str>>,
) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let body = json!({
        "state": column,
        "ordered_post_ids": ids,
        "column_states": column_states,
    });
    let req = Request::builder()
        .method(Method::PATCH)
        .uri("/api/posts/kanban-reorder")
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

/// A reorder of the Drafts column renumbers the ERROR posts too.
///
/// The error post is moved to the top of the column, so it must get
/// sort_order 1.
#[tokio::test]
async fn a_reorder_of_a_mixed_column_renumbers_the_error_posts() {
    let state = state().await;
    let draft_a = seed_post_in_state(&state, "draft").await;
    let draft_b = seed_post_in_state(&state, "draft").await;
    let errored = seed_post_in_state(&state, "error").await;

    // The error post is dragged to the top of Drafts.
    let (status, body) = reorder(
        &state,
        "draft",
        &[errored, draft_a, draft_b],
        Some(vec!["draft", "error"]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(
        sort_order_of(&state, errored).await,
        1,
        "the error post must be renumbered: {body}"
    );
    assert_eq!(sort_order_of(&state, draft_a).await, 2, "{body}");
    assert_eq!(sort_order_of(&state, draft_b).await, 3, "{body}");
}

/// A reorder of a single-state column still works.
#[tokio::test]
async fn a_reorder_of_a_single_state_column_renumbers_its_posts() {
    let state = state().await;
    let a = seed_post_in_state(&state, "queued").await;
    let b = seed_post_in_state(&state, "queued").await;

    let (status, body) = reorder(&state, "queued", &[b, a], None).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(sort_order_of(&state, b).await, 1, "{body}");
    assert_eq!(sort_order_of(&state, a).await, 2, "{body}");
}
