// ─── The feed analytics window must not filter everything away ──
//
// `external_posts.created_at` is an INTEGER epoch column, and every
// writer binds `EpochUtc`.
//
// `get_engagement_summary` bound the cutoff as a raw
// `Option<DateTime<Utc>>`. sqlx encodes a `DateTime` as RFC 3339
// TEXT, so the comparison was INTEGER >= TEXT. SQLite orders every
// INTEGER before every TEXT, so the predicate was FALSE for every row.
//
// The effect: the feed half of every analytics headline was
// structurally ZERO. The "Likes" card on /analytics showed only the
// own-post number, and the frontend's own comment - "Two sources
// contribute and they are ADDED" - described a sum in which one term
// could never be anything but zero.
//
// The same endpoint without `days` took the `NULL IS NULL`
// short-circuit and returned all-time totals, so the two spellings of
// the same request disagreed.
//
// These tests drive the real router and assert that a windowed
// request returns the rows inside the window and EXCLUDES the rows
// outside it.
//
// Run: cargo test --test feed_analytics_window_test
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

/// An imported post with `likes` engagement, `days_ago` days old.
///
/// The engagement row is keyed by the `external_posts` id - the FK is
/// `post_engagement.post_id -> external_posts.id`, NOT
/// `posts.id` (AGENTS.md 9.9).
async fn seed_imported_post(state: &AppState, days_ago: i64, likes: i64) -> Uuid {
    let external_id = Uuid::new_v4();
    let created = chrono::Utc::now() - chrono::Duration::days(days_ago);
    let created_epoch = created.timestamp();
    sqlx::query(
        "INSERT INTO external_posts (id, user_id, provider, platform_post_id, text,
             author_name, author_handle, created_at, url, media, metadata, imported_at)
         VALUES (?, ?, 'x', ?, 'an imported post', 'Someone', '@someone', ?, 'https://x.com/i/status/1',
             '[]', '{}', ?)",
    )
    .bind(external_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("x-{external_id}"))
    .bind(created_epoch)
    .bind(created_epoch)
    .execute(&state.db)
    .await
    .expect("seed external post");

    sqlx::query(
        "INSERT INTO post_engagement (id, post_id, likes, comments, shares, views, saves,
             quotes, reposts, replies, reactions, upvotes, downvotes, upvote_ratio, awards, raw,
             fetched_at, created_at, updated_at, impressions)
         VALUES (?, ?, ?, 0, 0, 0, 0, 0, 0, 0, '{}', 0, 0, NULL, 0, '{}', ?, ?, ?, 0)",
    )
    .bind(Uuid::new_v4())
    .bind(external_id)
    .bind(likes)
    .bind(created_epoch)
    .bind(created_epoch)
    .bind(created_epoch)
    .execute(&state.db)
    .await
    .expect("seed engagement");
    external_id
}

async fn get(state: &AppState, uri: &str) -> (StatusCode, String) {
    let token = create_token(DEFAULT_USER_ID, &state.config.jwt_secret).expect("token");
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
        .header(header::ORIGIN, &state.config.frontend_url)
        .body(Body::empty())
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

/// A windowed feed-analytics request must count the posts INSIDE the
/// window.
///
/// On the old code this returned 0 for every field: the cutoff was
/// bound as RFC 3339 TEXT against an INTEGER epoch column, and SQLite
/// orders every INTEGER before every TEXT.
#[tokio::test]
async fn a_windowed_request_counts_the_posts_inside_the_window() {
    let state = state().await;
    seed_imported_post(&state, 2, 40).await;
    seed_imported_post(&state, 5, 7).await;

    let (status, body) = get(&state, "/api/feed/analytics?days=30").await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let likes = json["total_likes"].as_i64().expect("total_likes");
    let posts = json["posts_with_engagement"]
        .as_i64()
        .expect("posts_with_engagement");

    assert!(
        likes >= 47,
        "a 30-day window must count the 40 and the 7, got {likes}: {body}"
    );
    assert!(
        posts >= 2,
        "a 30-day window must count both posts, got {posts}: {body}"
    );
}

/// A windowed request must EXCLUDE the posts outside the window.
///
/// The other half of the same rule: a 1-day window must not see a
/// 5-day-old post. If the filter were a no-op this would still pass,
/// so the test is the pair.
#[tokio::test]
async fn a_windowed_request_excludes_the_posts_outside_the_window() {
    let state = state().await;
    // A unique marker: a post that is 40 days old with a distinctive
    // like count, so the windowed total can be told apart from the
    // all-time total.
    seed_imported_post(&state, 40, 1000).await;

    let (status, body) = get(&state, "/api/feed/analytics?days=30").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let windowed = json["total_likes"].as_i64().expect("total_likes");

    let (status, body) = get(&state, "/api/feed/analytics").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: Value = serde_json::from_str(&body).expect("the body must be json");
    let all_time = json["total_likes"].as_i64().expect("total_likes");

    assert!(
        all_time >= windowed + 1000,
        "the 40-day-old post's 1000 likes must be in the all-time total \
         and OUT of the 30-day window: all_time {all_time}, windowed {windowed}"
    );
    assert!(
        windowed < all_time,
        "the window must actually exclude something: all_time {all_time}, \
         windowed {windowed}"
    );
}
