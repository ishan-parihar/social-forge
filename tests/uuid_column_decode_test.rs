// ─── A uuid column is not a string ────────────────────────────
//
// sqlx encodes a `Uuid` column as BLOB(16), not TEXT(36). 23 query
// sites cast the id with a `String` type override:
//
//     SELECT id as "id!: String", name, ... FROM automation_rules
//
// The override tells sqlx to decode the column as TEXT. The column
// actually holds a BLOB, so the decode fails at RUNTIME:
//
//     invalid utf-8 sequence of 1 bytes from index 2
//
// The `.sqlx/` offline cache records the DECLARED type (Text, because
// the column is declared TEXT in the migration), so the build passes
// and the failure only appears when the query actually runs. The
// automation rules API 500'd on its own create+list cycle; the CLI's
// `tags list` and `webhooks list` failed the same way.
//
// The fix is to decode the column as a `Uuid`, which accepts BOTH the
// TEXT and the BLOB representations.
//
// These tests drive the REAL router and the REAL query layer, so they
// fail against the old `id!: String` code with the utf-8 decode error.
// A test that re-implements the query locally would pass on the old
// code and guard nothing.
//
// Run: cargo test --test uuid_column_decode_test
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

/// One integration, owned by the local user.
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

/// One automation rule, owned by the local user.
async fn seed_rule(state: &AppState, integration_id: Uuid, name: &str) -> Uuid {
    let rule_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO automation_rules (id, user_id, integration_id, name, trigger_type,
             response_template, response_type) VALUES (?, ?, ?, ?, 'comment', 'hello', 'template')",
    )
    .bind(rule_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(name)
    .execute(&state.db)
    .await
    .expect("seed rule");
    rule_id
}

/// A rule name unique to this run, so a query that looks the row up by
/// name finds THIS run's row and not an earlier run's.
fn unique_name(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

async fn request(
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

/// GET /api/automation/rules must return the rules, with their ids
/// readable.
///
/// On the old code the handler's `SELECT id as "id!: String"` decoded
/// a BLOB as utf-8 and the endpoint answered 500 with
/// "invalid utf-8 sequence of 1 bytes from index 2".
#[tokio::test]
async fn listing_automation_rules_returns_the_rules_with_readable_ids() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let rule_id = seed_rule(&state, integration_id, "Auto-reply to launch mentions").await;

    let (status, body) = request(&state, Method::GET, "/api/automation/rules", None).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "GET /api/automation/rules must answer 200 - the uuid id column \
         decoded as a String is a runtime 500: {body}"
    );

    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let rules = json["rules"].as_array().expect("a rules array");
    // The tests share one database, so other runs' rules are in here too.
    // What matters is that THIS rule is present with its id intact.
    let ids: Vec<&str> = rules.iter().filter_map(|r| r["id"].as_str()).collect();
    assert!(
        ids.contains(&rule_id.to_string().as_str()),
        "the seeded rule must come back with its id intact: {body}"
    );
    let mine = rules
        .iter()
        .find(|r| r["id"] == rule_id.to_string())
        .expect("the seeded rule");
    assert_eq!(mine["name"], "Auto-reply to launch mentions", "{body}");
}

/// POST /api/automation/rules must answer 200 and return the new id.
///
/// This is the path that failed FIRST in production: the INSERT's
/// `RETURNING id as "id!: String"` decoded the row it had just
/// written, so the rule was CREATED and the request still answered
/// 500 - leaving an orphaned rule the founder could not see.
#[tokio::test]
async fn creating_an_automation_rule_answers_200_with_the_new_id() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;

    let (status, body) = request(
        &state,
        Method::POST,
        "/api/automation/rules",
        Some(serde_json::json!({
            "integration_id": integration_id.to_string(),
            "name": "Round-trip rule",
            "trigger_type": "comment",
            "response_template": "hello",
            "response_type": "template",
        })),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "POST /api/automation/rules must answer 200 - the old code wrote \
         the rule and then failed to read its id back: {body}"
    );

    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let id = json["id"].as_str().expect("the id must be a string");
    assert!(Uuid::parse_str(id).is_ok(), "the id must be a uuid: {body}");

    // The rule is really there - the old code created it and then 500'd.
    let (status, body) = request(&state, Method::GET, "/api/automation/rules", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(id),
        "the created rule must be readable back: {body}"
    );
}

/// A uuid column holds BLOB bytes, and the app only ever writes BLOB.
///
/// sqlx's `Uuid` decoder is `Uuid::from_slice(value.blob())` - it
/// requires EXACTLY 16 bytes and rejects the 36-character TEXT form
/// outright ("invalid length: expected 16 bytes, found 36"). So a
/// `Uuid` decode is not a "read either representation" fix; it is a
/// "read the representation sqlx writes" fix.
///
/// The TEXT form only ever appears in a database that something other
/// than sqlx wrote, and nothing in this app does that: every uuid
/// column is written by binding a `Uuid`, which sqlx encodes as 16
/// bytes. The one column that holds a uuid AS TEXT on purpose is
/// `post_set_channels.channel_id`, and it is read back as a String and
/// parsed (`src/api/sets.rs::load_channel_ids`).
///
/// So the contract this test holds is: a uuid column is a BLOB, and
/// the endpoint must read it as one.
#[tokio::test]
async fn a_uuid_id_column_holds_16_bytes_and_decodes_as_a_uuid() {
    let state = state().await;
    let integration_id = seed_integration(&state).await;
    let name = unique_name("blob row");
    let rule_id = seed_rule(&state, integration_id, &name).await;

    // The column really does hold the 16 raw bytes.
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT id FROM automation_rules WHERE user_id = ? AND name = ?")
            .bind(DEFAULT_USER_ID)
            .bind(&name)
            .fetch_one(&state.db)
            .await
            .expect("read the stored id");
    assert_eq!(
        stored.len(),
        16,
        "a uuid column must hold 16 bytes, got {} - a 36-character TEXT \
         id is a different shape and sqlx's Uuid decoder rejects it",
        stored.len()
    );
    assert_eq!(
        Uuid::from_slice(&stored).expect("the 16 bytes are a uuid"),
        rule_id
    );

    let (status, body) = request(&state, Method::GET, "/api/automation/rules", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(&rule_id.to_string()),
        "the rule must come back with its id intact: {body}"
    );
}
