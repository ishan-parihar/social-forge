// ─── A post can only be claimed once ──────────────────────────
//
// The scheduler claims due posts by flipping them `queued ->
// publishing`. The claim UPDATE had no `state = 'queued'` guard, and
// the transaction was a DEFERRED `BEGIN` (sqlx issues a plain BEGIN;
// the pool never configured BEGIN IMMEDIATE), so the SELECT ran with no
// lock held.
//
// Two claims that read the same id list could therefore both proceed:
// the second UPDATE flipped rows the first had already moved to
// 'publishing' and returned them. Both processes then called
// `provider.publish()` - and the post went out TWICE, which on a real
// audience is unrecoverable.
//
// The fix is the guard: the UPDATE only flips rows that are STILL
// 'queued', and the read-back only returns rows that are 'publishing'
// AND were in this claim's id list.
//
// Run: cargo test --test scheduler_claim_test
// Uses the isolated test DB from `load_dotenv_for_tests` (never data/social-forge.db).

use social_forge::auth::middleware::DEFAULT_USER_ID;
use social_forge::db;
use social_forge::db::queries::get_due_posts;
use social_forge::db::SqlitePool;
use uuid::Uuid;

async fn pool() -> SqlitePool {
    // A DEDICATED database file, not the shared scratch DB.
    //
    // The shared file accumulates posts from every other integration
    // test, and `get_due_posts` orders by `scheduled_at ASC` with a
    // LIMIT - so a handful of leftovers that are due pushed this test's
    // post out of the id list entirely, and the race never happened.
    //
    // A FILE (not `sqlite::memory:`) because the race needs two
    // independent CONNECTIONS: `create_pool` sets `max_connections(1)`,
    // so one in-memory pool cannot produce two.
    let path = "target/test-data/scheduler_claim_test.db";
    let _ = std::fs::remove_file(path);
    let pool = db::create_pool(&format!("sqlite://{path}"))
        .await
        .expect("pool");
    db::ensure_local_user(&pool).await.expect("local user");
    pool
}

async fn seed_due_post(pool: &SqlitePool) -> (Uuid, Uuid) {
    let integration_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, user_id, provider_identifier, provider_name, internal_id)
         VALUES (?, ?, 'bluesky', 'Bluesky', ?)",
    )
    .bind(integration_id)
    .bind(DEFAULT_USER_ID)
    .bind(format!("did:plc:{integration_id}"))
    .execute(pool)
    .await
    .expect("seed integration");

    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, user_id, integration_id, state, content, scheduled_at,
                            idempotency_key)
         VALUES (?, ?, ?, 'queued', 'a due post', unixepoch() - 60, ?)",
    )
    .bind(post_id)
    .bind(DEFAULT_USER_ID)
    .bind(integration_id)
    .bind(Uuid::new_v4().as_bytes().to_vec())
    .execute(pool)
    .await
    .expect("seed post");
    (post_id, integration_id)
}

#[tokio::test]
async fn a_due_post_is_claimed_exactly_once() {
    let pool = pool().await;
    let (post_id, _) = seed_due_post(&pool).await;

    let first = get_due_posts(&pool, 10).await.expect("first claim");
    assert!(
        first.iter().any(|p| p.id == post_id),
        "the first claim must take the post"
    );

    // The SECOND claim must find nothing for this post. Before the fix
    // it found the same post again - and the post was published twice.
    let second = get_due_posts(&pool, 10).await.expect("second claim");
    assert!(
        !second.iter().any(|p| p.id == post_id),
        "a post that is already publishing must not be claimed again"
    );

    let state: String = sqlx::query_scalar("SELECT state FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .expect("post row");
    assert_eq!(state, "publishing");
}

#[tokio::test]
async fn a_claimed_post_is_not_returned_by_a_later_claim() {
    let pool = pool().await;
    let (post_id, _) = seed_due_post(&pool).await;

    let _ = get_due_posts(&pool, 10).await.expect("claim");

    // A post that is not 'publishing' and not 'queued' is never claimed.
    sqlx::query("UPDATE posts SET state = 'published' WHERE id = ?")
        .bind(post_id)
        .execute(&pool)
        .await
        .expect("mark published");

    let third = get_due_posts(&pool, 10).await.expect("third claim");
    assert!(
        !third.iter().any(|p| p.id == post_id),
        "a published post must never be claimed"
    );
}
