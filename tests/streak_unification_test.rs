// ─── One streak measure: /api/streak and /api/analytics/cadence ───
//
// The TopBar's flame badge and the cadence card both show a "streak".
// The badge used to read `users.streak_days` - the scheduler's separate
// 24h-window counter, looked up by the hard-coded DEFAULT_USER_ID - so
// the two numbers could disagree across a midnight boundary, and on an
// instance whose users row is not exactly DEFAULT_USER_ID the badge
// read a user that owns no posts (0 forever).
//
// Now /api/streak computes through `analytics::calculate_streak` - the
// SAME zone-day rule the cadence card uses - and resolves the user from
// the session, like every other handler. These tests pin:
//   1. the two surfaces return the SAME number for the same zone;
//   2. the badge counts the SESSION's user, not a hard-coded id;
//   3. an unknown zone name is a loud 400, not a silent UTC streak.
//
// Run: cargo test --test streak_unification_test

use axum::http::{Method, StatusCode};
use serde_json::json;
use switchpost::api::{build_router, AppState};
use switchpost::auth::jwt::create_token;
use switchpost::auth::middleware::{DEFAULT_USER_ID, SESSION_COOKIE};
use switchpost::config::Config;
use switchpost::db;
use switchpost::realtime::Broadcaster;
use switchpost::social::registry::ProviderRegistry;
use tower::ServiceExt;
use uuid::Uuid;

/// A real router + pool on the isolated test DB (never the operator's).
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

/// Seed a published post directly with the given epoch, owned by the
/// given user. Published rows are the streak's only input, and the
/// create API drafts - publishing goes through real providers, which
/// no test wants. The integration row matches the calendar tests' seed.
async fn seed_published(state: &AppState, user_id: Uuid, epoch: i64) {
    let integration = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM integrations WHERE user_id = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .expect("integration lookup");
    let integration_id = match integration {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO integrations (id, user_id, provider_identifier, provider_name,
                                           internal_id, access_token, auth_method, created_at, updated_at)
                 VALUES (?, ?, 'x', 'X (Twitter)', ?, 'seed', 'cookie', unixepoch(), unixepoch())",
            )
            .bind(id)
            .bind(user_id)
            .bind(format!("seed-{id}"))
            .execute(&state.db)
            .await
            .expect("seed integration");
            id
        }
    };

    let post_id = Uuid::new_v4();
    let idem = format!("streak-test-{post_id}");
    sqlx::query(
        r#"INSERT INTO posts (id, user_id, integration_id, content, state, published_at, deleted_at, created_at, updated_at, idempotency_key, settings)
           VALUES (?, ?, ?, 'streak unification test post', 'published', ?, NULL, ?, ?, ?, '{}')"#,
    )
    .bind(post_id)
    .bind(user_id)
    .bind(integration_id)
    .bind(switchpost::db::types::EpochUtc::from(
        chrono::DateTime::from_timestamp(epoch, 0).unwrap(),
    ))
    .bind(switchpost::db::types::EpochUtc::from(chrono::Utc::now()))
    .bind(switchpost::db::types::EpochUtc::from(chrono::Utc::now()))
    .bind(idem)
    .execute(&state.db)
    .await
    .expect("seed published post");
}

async fn call_as(
    state: &AppState,
    method: Method,
    uri: &str,
    user_id: Uuid,
) -> (StatusCode, String) {
    let token = create_token(user_id, &state.config.jwt_secret).expect("token");
    let req = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header(axum::http::header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let res = build_router(state.clone())
        .oneshot(req)
        .await
        .expect("router call");
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&body).to_string())
}

/// A timestamp `n` days BEFORE the most recent midnight that has
/// already passed in EVERY zone the tests query.
///
/// The old helper anchored "today" at MIDDAY UTC. That is only in the
/// past when the clock is already past 12:00 UTC — at 00:35 UTC the
/// seed for "today" was 11 hours in the FUTURE, and in New York the
/// seed landed on a day the zone had not reached yet. `calculate_streak`
/// only accepts a streak that starts on today or yesterday, so a future
/// day measured a streak of 0: the suite went red every night and green
/// every afternoon.
///
/// Anchoring on the most recent PAST midnight makes every seeded day a
/// whole day in the past in every zone, so the measure is the same at
/// every hour of the day.
fn days_ago_at_midnight(n: i64) -> i64 {
    let today_midnight = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    (today_midnight - chrono::Duration::days(n)).timestamp()
}

/// The badge and the cadence card must return the SAME streak for the
/// same zone - one measure on both surfaces - and it must be nonzero
/// against real consecutive published days.
#[tokio::test]
async fn the_badge_and_the_cadence_card_show_the_same_streak() {
    let state = state().await;
    // The scratch DB persists across runs (it is per-BINARY, not per-run), so
    // previous runs' seeds are still in it: an earlier pass seeded Oct 19-22 and
    // this test then measured a 5-day streak against its own 3 new days and
    // failed. Delete this file's own published rows first, so the assertion
    // below is about the 3 days this test means to seed.
    sqlx::query(
        "DELETE FROM posts WHERE user_id = ? AND deleted_at IS NULL AND state = 'published'",
    )
    .bind(DEFAULT_USER_ID)
    .execute(&state.db)
    .await
    .expect("clear scratch published rows");

    // Three consecutive days ending today (UTC midnight anchor), so both
    // a UTC day and a New York day count them.
    for d in 0..3 {
        seed_published(&state, DEFAULT_USER_ID, days_ago_at_midnight(d)).await;
    }

    // ("cadence query suffix", "badge query suffix")
    for (cadence_suffix, badge_suffix) in [("", ""), ("&tz=America/New_York", "?tz=America/New_York")] {
        let (status, badge) = call_as(&state, Method::GET, &format!("/api/streak{badge_suffix}"), DEFAULT_USER_ID).await;
        assert_eq!(status, StatusCode::OK, "badge body: {badge}");
        let badge_streak: i64 =
            serde_json::from_str::<serde_json::Value>(&badge).unwrap()["streak_days"]
                .as_i64()
                .unwrap();

        let (status, cadence) = call_as(
            &state,
            Method::GET,
            &format!("/api/analytics/cadence?days=30{cadence_suffix}"),
            DEFAULT_USER_ID,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "cadence body: {cadence}");
        let cadence_streak: i64 =
            serde_json::from_str::<serde_json::Value>(&cadence).unwrap()["streak_days"]
                .as_i64()
                .unwrap();

        assert_eq!(
            badge_streak, cadence_streak,
            "the TopBar badge and the cadence card disagree ({badge_streak} vs {cadence_streak}) \
             for {cadence_suffix:?} - the two streak numbers on screen must be one measure"
        );
        assert_eq!(badge_streak, 3, "three consecutive days is a 3-day streak for {cadence_suffix:?}");
    }
}

/// The badge counts the SESSION's own published days - the
/// seeded-instance bug, pinned. A session whose user is NOT
/// DEFAULT_USER_ID must see ITS OWN posts: before the fix the handler
/// looked up the hard-coded constant, and a differently-seeded
/// instance's badge read a user that owned nothing (0 forever).
/// Per-test isolation: a fresh user row, its own session, its own
/// posts - independent of what any other test in this binary seeded.
#[tokio::test]
async fn the_badge_counts_the_session_users_own_published_days() {
    let state = state().await;
    // A user that is NOT the default constant - the exact shape of an
    // instance seeded with its own users row.
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Streak Test')")
        .bind(other)
        .bind(format!("{other}@streak.test"))
        .execute(&state.db)
        .await
        .expect("seed user");

    // No published posts for this user: 0.
    let (status, badge) = call_as(&state, Method::GET, "/api/streak", other).await;
    assert_eq!(status, StatusCode::OK, "badge body: {badge}");
    let streak: i64 = serde_json::from_str::<serde_json::Value>(&badge).unwrap()["streak_days"]
        .as_i64()
        .unwrap();
    assert_eq!(streak, 0, "no published posts for this session's user, no streak");

    // Publish today for THIS user: the streak becomes 1 for this user.
    seed_published(&state, other, days_ago_at_midnight(0)).await;
    let (status, badge) = call_as(&state, Method::GET, "/api/streak", other).await;
    assert_eq!(status, StatusCode::OK, "badge body: {badge}");
    let streak: i64 = serde_json::from_str::<serde_json::Value>(&badge).unwrap()["streak_days"]
        .as_i64()
        .unwrap();
    assert_eq!(
        streak, 1,
        "today's post by THIS session's user starts a 1-day streak for them"
    );
}

#[tokio::test]
async fn an_unknown_zone_is_a_loud_400_on_the_badge() {
    let state = state().await;
    let (status, body) = call_as(&state, Method::GET, "/api/streak?tz=Not/AZone", DEFAULT_USER_ID).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert!(
        body.contains("not a known IANA zone"),
        "the 400 must name the bad zone: {body}"
    );
}
