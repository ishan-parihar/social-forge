// ─── external_posts timestamps are INTEGER epoch ───────────────
//
// `external_posts.created_at` is declared INTEGER epoch, but the writer
// bound a `DateTime<Utc>`, which sqlx stores as RFC 3339 TEXT. SQLite
// keeps TEXT in an INTEGER-affinity column, and SQLite orders every
// INTEGER before every TEXT - so `created_at >= ?` with an INTEGER
// cutoff was ALWAYS TRUE.
//
// The effect: the 90-day window on the research page, the feed
// engagement summary and the posting-time histogram all let through
// posts of ANY age. The filter looked like it worked.
//
// These tests write through the real writer and read back through the
// real query, so they fail against the old TEXT storage.
//
// Run: cargo test --test external_posts_window_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/switchpost.db).

use chrono::{Duration, TimeZone, Utc};
use switchpost::auth::middleware::DEFAULT_USER_ID;
use switchpost::config::Config;
use switchpost::db;
use switchpost::db::queries::{insert_external_post, top_external_posts_by_engagement};
use switchpost::db::SqlitePool;
use uuid::Uuid;

async fn pool() -> SqlitePool {
    switchpost::config::load_dotenv_for_tests();
    let config = Config::from_env().expect("config");
    let pool = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    pool
}

fn days_ago(n: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap() - Duration::days(n)
}

/// One imported feed post, written through the real writer, with the
/// engagement row the ranking query joins.
async fn seed(pool: &SqlitePool, marker: &str, created_at: chrono::DateTime<Utc>, likes: i64) {
    let platform_post_id = format!("{marker}-{}", Uuid::new_v4());
    let inserted = insert_external_post(
        pool,
        DEFAULT_USER_ID,
        "x",
        &platform_post_id,
        marker,
        Some("Someone"),
        Some("@someone"),
        None,
        created_at,
        Some("https://x.com/someone/status/1"),
        &serde_json::json!([]),
        &serde_json::json!({}),
    )
    .await
    .expect("seed post");
    let external_post_id = inserted.expect("inserted").id;

    sqlx::query(
        "INSERT INTO post_engagement (id, post_id, likes, comments, shares)
         VALUES (?, ?, ?, 0, 0)",
    )
    .bind(Uuid::new_v4())
    .bind(external_post_id)
    .bind(likes)
    .execute(pool)
    .await
    .expect("seed engagement");
}

#[tokio::test]
async fn created_at_is_stored_as_an_integer_epoch() {
    let pool = pool().await;
    let platform_post_id = format!("epoch-type-{}", Uuid::new_v4());
    insert_external_post(
        &pool,
        DEFAULT_USER_ID,
        "x",
        &platform_post_id,
        "a competitor post",
        Some("Someone"),
        Some("@someone"),
        None,
        days_ago(1),
        Some("https://x.com/someone/status/1"),
        &serde_json::json!([]),
        &serde_json::json!({}),
    )
    .await
    .expect("seed post");

    let stored: (String,) =
        sqlx::query_as("SELECT typeof(created_at) FROM external_posts WHERE platform_post_id = ?")
            .bind(&platform_post_id)
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(
        stored.0, "integer",
        "the column is declared INTEGER and every reader compares it against an INTEGER cutoff; \
         TEXT storage made `created_at >= ?` always true"
    );
}

#[tokio::test]
async fn the_window_filter_excludes_older_posts() {
    let pool = pool().await;
    seed(&pool, "INSIDE-THE-WINDOW", days_ago(2), 40).await;
    seed(&pool, "OUTSIDE-THE-WINDOW", days_ago(200), 30).await;

    // The cutoff the research page uses: 90 days back.
    let cutoff = days_ago(90);
    let rows = top_external_posts_by_engagement(&pool, DEFAULT_USER_ID, cutoff, 100)
        .await
        .expect("query");

    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    assert!(
        texts.contains(&"INSIDE-THE-WINDOW"),
        "a post inside the window must be returned; got {texts:?}"
    );
    assert!(
        !texts.contains(&"OUTSIDE-THE-WINDOW"),
        "a post outside the window must NOT be returned - the filter was a no-op; got {texts:?}"
    );
}

#[tokio::test]
async fn a_post_outside_the_window_is_not_counted_at_all() {
    let pool = pool().await;
    seed(&pool, "RECENT-FOR-THE-COUNT", days_ago(2), 40).await;
    seed(&pool, "ANCIENT-FOR-THE-COUNT", days_ago(400), 90).await;

    let cutoff = days_ago(90);
    let rows = top_external_posts_by_engagement(&pool, DEFAULT_USER_ID, cutoff, 100)
        .await
        .expect("query");

    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    assert!(
        !texts.contains(&"ANCIENT-FOR-THE-COUNT"),
        "an ancient post must not appear in a 90-day ranking; got {texts:?}"
    );
}
