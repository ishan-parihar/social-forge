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
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use chrono::{Duration, TimeZone, Utc};
use social_forge::auth::middleware::DEFAULT_USER_ID;
use social_forge::config::Config;
use social_forge::db;
use social_forge::db::queries::{
    insert_external_post, list_all_external_posts, list_all_external_posts_with_engagement,
};
use social_forge::db::SqlitePool;
use uuid::Uuid;

async fn pool() -> SqlitePool {
    social_forge::config::load_dotenv_for_tests();
    let config = Config::from_env().expect("config");
    let pool = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    pool
}

fn days_ago(n: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap() - Duration::days(n)
}

/// One imported feed post, written through the real writer.
async fn seed(pool: &SqlitePool, platform_post_id: &str, created_at: chrono::DateTime<Utc>) {
    insert_external_post(
        pool,
        DEFAULT_USER_ID,
        "x",
        platform_post_id,
        "a competitor post",
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
}

#[tokio::test]
async fn created_at_is_stored_as_an_integer_epoch() {
    let pool = pool().await;
    let platform_post_id = format!("epoch-type-{}", Uuid::new_v4());
    seed(&pool, &platform_post_id, days_ago(1)).await;

    let stored: (String,) = sqlx::query_as(
        "SELECT typeof(created_at) FROM external_posts WHERE platform_post_id = ?",
    )
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
    let recent_id = format!("window-recent-{}", Uuid::new_v4());
    let old_id = format!("window-old-{}", Uuid::new_v4());
    seed(&pool, &recent_id, days_ago(2)).await;
    seed(&pool, &old_id, days_ago(200)).await;

    // The cutoff the research page uses: 90 days back.
    let cutoff = days_ago(90);
    let ids = list_all_external_posts_with_engagement(
        &pool,
        DEFAULT_USER_ID,
        None,
        Some(cutoff),
        100,
    )
    .await
    .expect("query");

    let platform_ids: Vec<String> = ids.iter().map(|p| p.platform_post_id.clone()).collect();
    assert!(
        platform_ids.contains(&recent_id),
        "a post inside the window must be returned"
    );
    assert!(
        !platform_ids.contains(&old_id),
        "a post outside the window must NOT be returned - the filter was a no-op"
    );
}

#[tokio::test]
async fn the_cursor_filter_excludes_newer_posts() {
    let pool = pool().await;
    let older_id = format!("cursor-older-{}", Uuid::new_v4());
    let newer_id = format!("cursor-newer-{}", Uuid::new_v4());
    seed(&pool, &older_id, days_ago(10)).await;
    seed(&pool, &newer_id, days_ago(1)).await;

    let cutoff = days_ago(5);
    let ids = list_all_external_posts(&pool, DEFAULT_USER_ID, None, Some(cutoff), 100)
        .await
        .expect("query");

    let platform_ids: Vec<String> = ids.iter().map(|p| p.platform_post_id.clone()).collect();
    assert!(
        platform_ids.contains(&older_id),
        "a post before the cursor must be returned"
    );
    assert!(
        !platform_ids.contains(&newer_id),
        "a post after the cursor must NOT be returned"
    );
}
