// ─── Research aggregation ──────────────────────────────────────
// The /research read surface aggregates four sections from data the app
// already stores. This walks the DB layer directly: the queries it calls,
// on rows seeded to cover each section plus the empty-DB case.
//
// Run: cargo test --test research_aggregate_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use chrono::{DateTime, TimeZone, Utc};
use social_forge::db;
use social_forge::db::queries::{
    engagement_histogram_by_slot, top_external_posts_by_engagement,
    top_own_posts_by_engagement,
};
use social_forge::db::SqlitePool;
use uuid::Uuid;

async fn pool() -> SqlitePool {
    social_forge::config::load_dotenv_for_tests();
    let config = social_forge::config::Config::from_env().expect("config");
    let pool = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    pool
}

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0).unwrap()
}

async fn fresh_user(pool: &SqlitePool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Researcher')")
        .bind(id)
        .bind(format!("{id}@research.test"))
        .execute(pool)
        .await
        .expect("seed user");
    id
}

async fn integration(pool: &SqlitePool, user_id: Uuid, provider: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(provider)
    .bind(provider)
    .bind(format!("internal-{id}"))
    .execute(pool)
    .await
    .expect("seed integration");
    id
}

/// A published own post, with its cached per-post analytics written the same
/// way `/api/analytics/post/{id}` does: a JSON series list keyed by
/// platform_post_id.
#[expect(
    clippy::too_many_arguments,
    reason = "test fixture: one parameter per seeded column"
)]
async fn own_post(
    pool: &SqlitePool,
    user_id: Uuid,
    content: &str,
    provider: &str,
    published: DateTime<Utc>,
    likes: i64,
    comments: i64,
    shares: i64,
) -> Uuid {
    let integration_id = integration(pool, user_id, provider).await;
    let id = Uuid::new_v4();
    let platform_post_id = format!("pp-{id}");
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key,
                            platform_post_id, published_at)
         VALUES (?, ?, ?, 'published', ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(integration_id)
    .bind(content)
    .bind(Uuid::new_v4())
    .bind(&platform_post_id)
    .bind(published.timestamp())
    .execute(pool)
    .await
    .expect("seed own post");

    // The cached analytics payload: provider-shaped series, all with a single
    // dated point, `total` as a string (that is how every provider emits it).
    let data = serde_json::json!([
        { "label": "Likes", "data": [{ "total": likes.to_string(), "date": "2026-10-19" }],
          "percentage_change": 0.0 },
        { "label": "Comments", "data": [{ "total": comments.to_string(), "date": "2026-10-19" }],
          "percentage_change": 0.0 },
        { "label": "Shares", "data": [{ "total": shares.to_string(), "date": "2026-10-19" }],
          "percentage_change": 0.0 },
        // Reach must not count as engagement — excluded by the query.
        { "label": "Impressions", "data": [{ "total": "999999", "date": "2026-10-19" }],
          "percentage_change": 0.0 },
    ]);
    sqlx::query(
        "INSERT INTO analytics_cache (id, user_id, provider, platform_post_id, data)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(provider)
    .bind(&platform_post_id)
    .bind(data.to_string())
    .execute(pool)
    .await
    .expect("seed analytics cache");
    id
}

/// An imported competitor post with engagement rows, the way the feed
/// importer stores them.
#[expect(
    clippy::too_many_arguments,
    reason = "test fixture: one parameter per seeded column"
)]
async fn external_post(
    pool: &SqlitePool,
    user_id: Uuid,
    provider: &str,
    text: &str,
    created: DateTime<Utc>,
    likes: i32,
    comments: i32,
    shares: i32,
    hidden: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO external_posts (id, user_id, provider, platform_post_id, text, created_at, hidden_at)
         VALUES (?, ?, ?, ?, ?, ?, CASE WHEN ? THEN 12345 END)",
    )
    .bind(id)
    .bind(user_id)
    .bind(provider)
    .bind(format!("ext-{id}"))
    .bind(text)
    .bind(created.timestamp())
    .bind(hidden)
    .execute(pool)
    .await
    .expect("seed external post");
    post_engagement(pool, id, likes, comments, shares).await;
    id
}

async fn post_engagement(
    pool: &SqlitePool,
    post_id: Uuid,
    likes: i32,
    comments: i32,
    shares: i32,
) {
    sqlx::query(
        "INSERT INTO post_engagement (id, post_id, likes, comments, shares)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (post_id) DO UPDATE SET likes = EXCLUDED.likes, comments = EXCLUDED.comments,
           shares = EXCLUDED.shares",
    )
    .bind(Uuid::new_v4())
    .bind(post_id)
    .bind(likes)
    .bind(comments)
    .bind(shares)
    .execute(pool)
    .await
    .expect("seed engagement");
}

/// One full research fixture: a seeded user with their own posts and
/// competitor posts covering every section.
struct Fixture {
    pool: SqlitePool,
    user_id: Uuid,
}

async fn fixture() -> Fixture {
    let pool = pool().await;
    let user_id = fresh_user(&pool).await;

    // (a) Own posts. The 18:00 slot beats the 09:00 slot on
    // engagement-per-post (10+5+5 = 20) vs (40+0+0 = 40 over 4 posts).
    own_post(&pool, user_id, "Robotics teardown #1", "youtube", at(1, 18), 10, 5, 5).await;
    own_post(&pool, user_id, "Robotics teardown #2", "youtube", at(2, 9), 10, 0, 0).await;
    own_post(&pool, user_id, "Robotics teardown #3", "youtube", at(3, 9), 10, 0, 0).await;
    own_post(&pool, user_id, "Robotics teardown #4", "youtube", at(4, 9), 10, 0, 0).await;
    own_post(&pool, user_id, "Robotics teardown #5", "youtube", at(5, 9), 10, 0, 0).await;

    // (b) Competitor posts, one high, one zero (excluded), one hidden.
    external_post(&pool, user_id, "x", "swarm robotics for warehouses", at(6, 12), 200, 40, 20, false).await;
    external_post(&pool, user_id, "x", "quiet week, nothing happened", at(6, 13), 0, 0, 0, false).await;
    external_post(&pool, user_id, "x", "hidden swarm post should not rank", at(6, 14), 500, 10, 5, true).await;
    external_post(&pool, user_id, "x", "swarm robotics revisited", at(7, 12), 40, 8, 4, false).await;

    Fixture { pool, user_id }
}

#[tokio::test]
async fn own_posts_rank_by_engagement_and_carry_provider() {
    let f = fixture().await;
    let rows = top_own_posts_by_engagement(&f.pool, f.user_id, at(1, 0), 10)
        .await
        .expect("own posts");

    assert_eq!(rows.len(), 5, "all five own posts rank: {rows:?}");
    // 10+5+5 is the top; ties broken by recency, then the 4x 10s.
    assert_eq!(rows[0].content, "Robotics teardown #1");
    assert_eq!(rows[0].engagement, 20, "likes+comments+shares, not impressions");
    assert_eq!(rows[0].provider, "youtube");
    assert_eq!(rows[0].published_at, Some(at(1, 18)));
}

#[tokio::test]
async fn own_posts_respect_the_window() {
    let f = fixture().await;
    // A cutoff after the first post excludes it and the ones before it.
    let rows = top_own_posts_by_engagement(&f.pool, f.user_id, at(3, 0), 10)
        .await
        .expect("windowed own posts");
    assert!(
        rows.iter().all(|p| p.published_at >= Some(at(3, 0))),
        "window floor must hold: {rows:?}"
    );
    assert_eq!(rows.len(), 3);
}

#[tokio::test]
async fn own_post_without_cached_analytics_is_excluded_from_ranking() {
    let f = fixture().await;
    // A published post with no cached engagement has no engagement to rank
    // on; it must not be reported as a 0-engagement "top" post.
    let integration_id = integration(&f.pool, f.user_id, "x").await;
    let quiet = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, idempotency_key,
                            platform_post_id, published_at)
         VALUES (?, ?, ?, 'published', 'nobody saw this', ?, 'quiet-post', ?)",
    )
    .bind(quiet)
    .bind(f.user_id)
    .bind(integration_id)
    .bind(Uuid::new_v4())
    .bind(at(8, 12).timestamp())
    .execute(&f.pool)
    .await
    .expect("seed quiet post");

    let rows = top_own_posts_by_engagement(&f.pool, f.user_id, at(1, 0), 10)
        .await
        .expect("own posts");
    assert!(
        rows.iter().all(|p| p.post_id != quiet),
        "a post with no engagement metrics must not appear as a top post: {rows:?}"
    );
}

#[tokio::test]
async fn external_posts_rank_by_engagement_and_drop_zero_and_hidden() {
    let f = fixture().await;
    let rows = top_external_posts_by_engagement(&f.pool, f.user_id, at(6, 0), 10)
        .await
        .expect("external posts");

    assert_eq!(rows.len(), 2, "zero-engagement and hidden rows are excluded: {rows:?}");
    assert_eq!(rows[0].text, "swarm robotics for warehouses");
    assert_eq!(rows[0].engagement, 260);
    assert_eq!(rows[1].text, "swarm robotics revisited");
    assert_eq!(rows[1].engagement, 52);
}

#[tokio::test]
async fn external_gap_words_appear_in_many_competitor_posts_not_in_own() {
    let f = fixture().await;
    // `swarm` appears in both ranking competitor posts.
    // `robotics` appears in competitor posts AND in the founder's own posts
    // (which seed "Robotics teardown" content), so it is not a gap — the gap
    // computation happens in the API layer on the text of both sides, and the
    // query supplies the competitor text for it.
    let rows = top_external_posts_by_engagement(&f.pool, f.user_id, at(6, 0), 10)
        .await
        .expect("external posts");
    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| t.contains("swarm")),
        "fixture must carry the gap topic: {texts:?}"
    );
}

#[tokio::test]
async fn timing_histogram_respects_the_window_and_averages_per_post() {
    let f = fixture().await;
    // Seed one external post per slot with engagement, the way the feed
    // refresher does, so the histogram has something to bucket.
    // The 18:00 post carries 50 engagement; the 09:00 post only 10, and six
    // more 09:00 posts drag that slot's average down further.
    external_post(&f.pool, f.user_id, "x", "a", at(14, 18), 30, 10, 10, false).await;
    // Six posts at the same weekday+hour, each weak. The histogram buckets
    // them together, so the slot carries volume without quality.
    for _ in 0..6 {
        external_post(&f.pool, f.user_id, "x", "b", at(15, 9), 10, 0, 0, false).await;
    }

    // A window starting after the 18:00 post must drop that slot entirely,
    // which is the only way to prove the cutoff is applied rather than the
    // pre-cutoff behaviour silently surviving.
    let after_18 = engagement_histogram_by_slot(&f.pool, f.user_id, None, Some(at(20, 0)))
        .await
        .expect("histogram");
    assert!(
        after_18.is_empty(),
        "a window that starts after every post yields nothing: {after_18:?}"
    );

    let all = engagement_histogram_by_slot(&f.pool, f.user_id, None, None)
        .await
        .expect("histogram");
    let by_slot: std::collections::HashMap<(i32, i32), (i64, i64)> = all
        .into_iter()
        .map(|(_, wd, h, posts, eng)| ((wd, h), (posts, eng)))
        .collect();
    // Weekdays are SQLite's %w (0=Sunday..6=Saturday) over the platform
    // post timestamp: 2026-10-14 is a Wednesday (3) at 18:00, and every
    // 09:00 post below is 2026-10-15, a Thursday (4).
    let (posts_18, eng_18) = by_slot[&(3, 18)];
    let (posts_9, eng_9) = by_slot[&(4, 9)];
    assert_eq!(eng_18, 50, "likes+comments+shares sum");
    // Volume is not quality: the 09:00 slot has more posts but a worse average.
    assert!(posts_9 > posts_18, "09:00 has more volume: {posts_9} vs {posts_18}");
    let avg_18 = eng_18 as f64 / posts_18 as f64;
    let avg_9 = eng_9 as f64 / posts_9 as f64;
    assert!(
        avg_18 > avg_9,
        "engagement-per-post must rank 18:00 above 09:00: {avg_18} vs {avg_9}"
    );
}

#[tokio::test]
async fn empty_database_yields_empty_sections() {
    let pool = pool().await;
    let lonely = fresh_user(&pool).await;

    let own = top_own_posts_by_engagement(&pool, lonely, at(1, 0), 10)
        .await
        .expect("empty own");
    assert!(own.is_empty(), "no own posts: {own:?}");

    let external = top_external_posts_by_engagement(&pool, lonely, at(1, 0), 10)
        .await
        .expect("empty external");
    assert!(external.is_empty(), "no competitor posts: {external:?}");

    let slots = engagement_histogram_by_slot(&pool, lonely, None, Some(at(1, 0)))
        .await
        .expect("empty histogram");
    assert!(slots.is_empty(), "no timing signal: {slots:?}");

    let feeds: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT rp.id FROM rss_posts rp
          JOIN rss_feeds rf ON rp.feed_id = rf.id
         WHERE rf.user_id = ?",
    )
    .bind(lonely)
    .fetch_all(&pool)
    .await
    .expect("empty feeds");
    assert!(feeds.is_empty(), "no feed items: {feeds:?}");
}
