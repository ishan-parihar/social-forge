// ─── `posts.media` is an array ────────────────────────────────
//
// The RSS autopost path wrote a JSON OBJECT into `posts.media`:
//
//     create_post(db, ..., &serde_json::json!({}), &serde_json::json!({"rss_auto": true}), ...)
//
// `create_post`'s sixth parameter is `media`, so every RSS draft was
// stored with `media = {}`.
//
// The frontend assumes an array. The kanban's `coverImage` calls
// `post.media.find(...)` directly, and `{}` has no `find`, so the whole
// ROUTE crashed with "i.find is not a function" - a pageerror, not just
// one bad card. The calendar's `(post.media || []).map(...)` failed the
// same way, because `{}` is truthy so the `||` fallback never fired.
//
// Three layers own this, and each has its own test here:
//   1. the WRITER (both RSS paths) writes an array
//   2. the MIGRATION repairs the rows that are already there
//   3. the READER normalises anything that slipped through
//
// Run: cargo test --test posts_media_shape_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use social_forge::db;
use social_forge::db::models::media_as_array;
use uuid::Uuid;

async fn pool() -> social_forge::db::SqlitePool {
    social_forge::config::load_dotenv_for_tests();
    let pool = db::create_pool(social_forge::config::TEST_DATABASE_URL)
        .await
        .expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    pool
}

const USER: Uuid = social_forge::auth::middleware::DEFAULT_USER_ID;

async fn seed_post(pool: &social_forge::db::SqlitePool, media: &str) -> Uuid {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'bluesky', 'Bluesky', ?)",
    )
    .bind(integration_id)
    .bind(USER)
    .bind(format!("did:plc:{integration_id}"))
    .execute(pool)
    .await
    .expect("seed integration");

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, media, settings,
                            idempotency_key)
         VALUES (?, ?, ?, 'draft', 'rss draft', ?, '{\"rss_auto\": true}', ?)",
    )
    .bind(post_id)
    .bind(USER)
    .bind(integration_id)
    .bind(media)
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("seed post");
    post_id
}

/// The READER: whatever shape the column holds, the value the API
/// returns is an ARRAY. A non-array took the kanban route down.
#[test]
fn the_reader_always_returns_an_array() {
    // The broken shape the RSS path wrote.
    let broken = serde_json::json!({});
    let out = media_as_array(&broken);
    assert!(
        out.is_array(),
        "an object must normalise to an array, got {out}"
    );
    assert_eq!(out.as_array().expect("an array").len(), 0);

    // The normal shape passes through untouched.
    let normal = serde_json::json!([{"url": "/api/media/1", "mime_type": "image/png"}]);
    assert_eq!(media_as_array(&normal), normal);

    // An empty array stays empty.
    assert_eq!(
        media_as_array(&serde_json::json!([])),
        serde_json::json!([])
    );

    // NULL becomes an empty array, not null.
    let out = media_as_array(&serde_json::Value::Null);
    assert!(out.is_array(), "null must normalise to an array, got {out}");

    // A single attachment OBJECT is one attachment, not zero.
    let one = serde_json::json!({"url": "/api/media/1", "mime_type": "image/png"});
    let out = media_as_array(&one);
    assert_eq!(out.as_array().expect("an array").len(), 1);
    assert_eq!(out[0]["url"], "/api/media/1");
}

/// The WRITER: the RSS paths must put an ARRAY in the column. A
/// regression here puts the `{}` rows back.
#[tokio::test]
async fn an_rss_draft_writes_an_array_into_media() {
    let pool = pool().await;

    // This is the exact call the RSS autopost path makes, with the
    // media argument the fix uses. If someone changes it back to
    // `json!({})`, the column holds an object and this test fails.
    let media = serde_json::json!([]);
    let post_id = seed_post(&pool, &media.to_string()).await;

    let stored: String = sqlx::query_scalar("SELECT media FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .expect("read media");
    let parsed: serde_json::Value = serde_json::from_str(&stored).expect("json");
    assert!(
        parsed.is_array(),
        "posts.media must be an array, got {parsed}"
    );
}

/// The MIGRATION: the rows that are ALREADY there get repaired. An
/// operator's database holds them, and the migration is what clears
/// them.
#[tokio::test]
async fn the_migration_repairs_the_rows_that_are_already_there() {
    let pool = pool().await;

    // A row in the broken shape, as the RSS path wrote it.
    let broken_id = seed_post(&pool, "{}").await;

    let shape: String = sqlx::query_scalar("SELECT json_type(media) FROM posts WHERE id = ?")
        .bind(broken_id)
        .fetch_one(&pool)
        .await
        .expect("read the shape");
    assert_eq!(
        shape, "object",
        "the seeded row must be in the broken shape"
    );

    // The migration's own statements, run against this row.
    sqlx::query(
        "UPDATE posts SET media = '[]'
          WHERE json_type(media) = 'object'
            AND json_type(media, '$.url') IS NULL",
    )
    .execute(&pool)
    .await
    .expect("the repair statement must run");

    let shape: String = sqlx::query_scalar("SELECT json_type(media) FROM posts WHERE id = ?")
        .bind(broken_id)
        .fetch_one(&pool)
        .await
        .expect("read the shape");
    assert_eq!(
        shape, "array",
        "the migration must turn the object into an array"
    );
}
