// ─── Activity feed: merge + auth-gate ────────────────────────
// `GET /api/activity` is the only surface that reads the three append-only
// tables (events_log, automation_logs, publish_attempts) as one timeline.
// Two properties matter and neither is visible from a single table:
//
//   1. the merge returns rows from ALL THREE sources, newest first, and
//   2. the route is auth-gated — two of the three tables have no user_id
//      column, so the route reaches their rows only through an FK join,
//      and a hole in that join leaks another user's rows.
//
// Run: cargo test --test activity_feed_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/switchpost.db).

use axum::body::Body;
use axum::http::header;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;
use switchpost::api::{activity, build_router, AppState};
use switchpost::auth::jwt::create_token;
use switchpost::auth::middleware::{DEFAULT_USER_ID, SESSION_COOKIE};
use switchpost::config::Config;
use switchpost::db;
use switchpost::db::SqlitePool;
use switchpost::realtime::Broadcaster;
use switchpost::social::registry::ProviderRegistry;
use tower::ServiceExt;
use uuid::Uuid;

async fn state() -> AppState {
    switchpost::config::load_dotenv_for_tests();
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

/// Small accessors so a missing field reads as a failed assert rather than
/// a panic inside a helper (tests already `expect` their fixtures).
fn row_field<'a>(row: &'a Value, key: &str) -> &'a Value {
    row.get(key).unwrap_or(&Value::Null)
}

/// Three sources, three distinct timestamps. The calls below seed them so
/// that the expected merge order is:
///   publish_attempts (newest) → events_log → automation_logs (oldest)
async fn seed_user(pool: &SqlitePool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Activity')")
        .bind(id)
        .bind(format!("{id}@activity.test"))
        .execute(pool)
        .await
        .expect("seed user");
    id
}

async fn seed_integration(pool: &SqlitePool, user_id: Uuid, provider: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(provider)
    .bind(provider)
    .bind(format!("internal:{id}"))
    .execute(pool)
    .await
    .expect("seed integration");
    id
}

async fn seed_post(pool: &SqlitePool, user_id: Uuid, integration_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key)
         VALUES (?, ?, ?, 'published', 'activity probe', ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("seed post");
    id
}

/// One `publish_attempts` row. `at` is the epoch second it is stamped with.
async fn seed_publish_attempt(
    pool: &SqlitePool,
    post_id: Uuid,
    attempt_number: i64,
    status: &str,
    at: i64,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO publish_attempts (id, post_id, attempt_number, status, error_message,
                                       started_at, created_at)
         VALUES (?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(id)
    .bind(post_id)
    .bind(attempt_number)
    .bind(status)
    .bind(at)
    .bind(at)
    .execute(pool)
    .await
    .expect("seed publish attempt");
    id
}

/// One `automation_logs` row, reached through a freshly created rule.
async fn seed_automation_log(
    pool: &SqlitePool,
    user_id: Uuid,
    integration_id: Uuid,
    status: &str,
    at: i64,
) -> (Uuid, Uuid) {
    let rule_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO automation_rules (id, user_id, integration_id, name, trigger_type,
                                       response_template, response_type)
         VALUES (?, ?, ?, 'activity probe rule', 'comment', 'hi', 'template')",
    )
    .bind(rule_id)
    .bind(user_id)
    .bind(integration_id)
    .execute(pool)
    .await
    .expect("seed automation rule");

    let log_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO automation_logs (id, rule_id, trigger_id, trigger_type, response,
                                      status, error_message, created_at)
         VALUES (?, ?, 'trigger-1', 'comment', 'hi', ?, NULL, ?)",
    )
    .bind(log_id)
    .bind(rule_id)
    .bind(status)
    .bind(at)
    .execute(pool)
    .await
    .expect("seed automation log");
    (log_id, rule_id)
}

/// One `events_log` row, written the way the broadcaster writes it.
async fn seed_event(
    pool: &SqlitePool,
    user_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
    at: i64,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO events_log (id, user_id, event_type, payload, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(event_type)
    .bind(serde_json::to_string(&payload).expect("payload"))
    .bind(at)
    .execute(pool)
    .await
    .expect("seed event");
    id
}

/// Timestamps used by the fixtures, ordered newest to oldest. All are
/// within the default 24h window of `now`, and all distinct.
fn now_at(offset_secs: i64) -> i64 {
    chrono::Utc::now().timestamp() - offset_secs
}

/// Reads the feed through the real handler, without the auth middleware.
/// The gating behaviour is covered separately by the router-level test.
async fn feed_rows(state: &AppState, user_id: Uuid) -> Vec<Value> {
    let query = activity::ActivityQuery {
        kind: "all".to_string(),
        range: "7d".to_string(),
        provider: None,
        before: None,
    };
    let auth = switchpost::auth::middleware::AuthenticatedUser { user_id };
    let res = activity::list(
        axum::extract::State(state.clone()),
        auth,
        axum::extract::Query(query),
    )
    .await
    .expect("activity list");

    let body = serde_json::to_value(res.0).expect("serialize");
    body.get("rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn merge_returns_all_three_sources_newest_first() {
    let state = state().await;
    let pool = &state.db;
    let user_id = seed_user(pool).await;
    let integration_id = seed_integration(pool, user_id, "bluesky").await;
    let post_id = seed_post(pool, user_id, integration_id).await;

    // Newest first: publish attempt, then event, then automation log.
    let at_pub = now_at(300);
    let at_event = now_at(3600);
    let at_auto = now_at(7200);

    let pub_id = seed_publish_attempt(pool, post_id, 1, "success", at_pub).await;
    let event_id = seed_event(
        pool,
        user_id,
        "post_published",
        serde_json::json!({"post_id": post_id.to_string(), "provider": "bluesky"}),
        at_event,
    )
    .await;
    let (auto_id, rule_id) =
        seed_automation_log(pool, user_id, integration_id, "sent", at_auto).await;

    let rows = feed_rows(&state, user_id).await;
    assert_eq!(rows.len(), 3, "one row per seeded source, got: {rows:?}");

    // Newest first, by the `at` field.
    assert_eq!(row_field(&rows[0], "kind"), "publish");
    assert_eq!(row_field(&rows[1], "kind"), "event");
    assert_eq!(row_field(&rows[2], "kind"), "automation");

    let ats: Vec<i64> = rows
        .iter()
        .map(|r| row_field(r, "at").as_i64().expect("at"))
        .collect();
    assert_eq!(ats, vec![at_pub, at_event, at_auto]);

    // ── publish row ──
    assert_eq!(
        row_field(&rows[0], "id"),
        &Value::String(format!("publish:{pub_id}"))
    );
    assert_eq!(
        row_field(&rows[0], "post_id"),
        &Value::String(post_id.to_string())
    );
    assert_eq!(
        row_field(&rows[0], "provider"),
        &Value::String("bluesky".into())
    );
    assert_eq!(row_field(&rows[0], "ok"), &Value::Bool(true));
    assert!(
        row_field(&rows[0], "summary")
            .as_str()
            .unwrap_or_default()
            .contains("succeeded"),
        "summary should read as a sentence, got {:?}",
        row_field(&rows[0], "summary")
    );

    // ── event row ──
    assert_eq!(
        row_field(&rows[1], "id"),
        &Value::String(format!("event:{event_id}"))
    );
    assert_eq!(
        row_field(&rows[1], "provider"),
        &Value::String("bluesky".into())
    );
    assert_eq!(row_field(&rows[1], "ok"), &Value::Bool(true));

    // ── automation row ──
    // `automation_logs` has no user_id column, so this row existing at all
    // proves the join back through `automation_rules` scoped it correctly.
    assert_eq!(
        row_field(&rows[2], "id"),
        &Value::String(format!("automation:{auto_id}"))
    );
    assert_eq!(
        row_field(&rows[2], "rule_id"),
        &Value::String(rule_id.to_string())
    );
    assert_eq!(row_field(&rows[2], "ok"), &Value::Bool(true));
}

#[tokio::test]
async fn failed_rows_report_ok_false() {
    let state = state().await;
    let pool = &state.db;
    let user_id = seed_user(pool).await;
    let integration_id = seed_integration(pool, user_id, "bluesky").await;
    let post_id = seed_post(pool, user_id, integration_id).await;

    seed_publish_attempt(pool, post_id, 1, "failed", now_at(300)).await;
    seed_event(
        pool,
        user_id,
        "post_failed",
        serde_json::json!({"post_id": post_id.to_string()}),
        now_at(600),
    )
    .await;
    seed_automation_log(pool, user_id, integration_id, "failed", now_at(900)).await;

    let rows = feed_rows(&state, user_id).await;
    assert_eq!(rows.len(), 3);
    for row in &rows {
        assert_eq!(
            row_field(row, "ok"),
            &Value::Bool(false),
            "{:?} should be reported as not ok",
            row_field(row, "summary")
        );
    }
}

#[tokio::test]
async fn another_users_rows_are_excluded() {
    let state = state().await;
    let pool = &state.db;

    let owner = seed_user(pool).await;
    let stranger = seed_user(pool).await;
    let integration_id = seed_integration(pool, owner, "bluesky").await;
    let post_id = seed_post(pool, owner, integration_id).await;

    // All three sources, all belonging to `stranger`. `automation_logs` and
    // `publish_attempts` carry no user_id, so this only holds if the joins
    // scope them.
    seed_publish_attempt(pool, post_id, 1, "success", now_at(300)).await;
    seed_event(
        pool,
        stranger,
        "post_published",
        serde_json::json!({}),
        now_at(600),
    )
    .await;
    seed_automation_log(
        pool,
        stranger,
        seed_integration(pool, stranger, "bluesky").await,
        "sent",
        now_at(900),
    )
    .await;

    // Read as the owner: the publish attempt is the owner's post, so it IS
    // theirs — but the event and automation rows are not.
    let rows = feed_rows(&state, owner).await;
    let kinds: Vec<&str> = rows
        .iter()
        .map(|r| row_field(r, "kind").as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        kinds,
        vec!["publish"],
        "only the owner's own rows: {kinds:?}"
    );

    // Read as a user with nothing at all.
    let empty = feed_rows(&state, seed_user(pool).await).await;
    assert!(
        empty.is_empty(),
        "an unrelated user sees nothing: {empty:?}"
    );
}

#[tokio::test]
async fn read_only_route_rejects_writes_and_requires_a_session() {
    let state = state().await;

    // Anonymous: the auth middleware runs before method routing, so a
    // write without a session is rejected outright.
    for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
        let res = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri("/api/activity")
                    .header(header::ORIGIN, "http://localhost:6543")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router");
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "{method} without a session must not reach the handler"
        );
    }

    // With a session: the route registers GET only, so every write verb is
    // 405 — never 201/204. This is the assertion that proves the surface is
    // read-only rather than merely gated.
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
        let res = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri("/api/activity")
                    .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
                    .header(header::ORIGIN, &state.config.frontend_url)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router");
        assert_eq!(
            res.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{method} must not write to the activity surface"
        );
    }

    // GET without a session is rejected by the auth middleware, so an
    // anonymous caller cannot read the merged feed.
    let res = build_router(state)
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/activity")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
