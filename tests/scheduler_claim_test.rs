// ─── A post can only be claimed once ──────────────────────────
//
// The scheduler claims due posts by flipping them `queued ->
// publishing`. The claim UPDATE had no `state = 'queued'` guard, and
// the transaction is a DEFERRED `BEGIN` (sqlx issues a plain BEGIN;
// the pool never configured BEGIN IMMEDIATE), so the SELECT of the due
// ids ran with NO lock held.
//
// Two claims that read the same id list could therefore both proceed:
// the second UPDATE flipped rows the first had already moved to
// 'publishing' and returned them. Both processes then called
// `provider.publish()` - and the post went out TWICE, which on a real
// audience is unrecoverable.
//
// The fix is the guard: the UPDATE only flips rows that are STILL
// 'queued', and `RETURNING id` tells the claim exactly which rows IT
// flipped.
//
// The test calls the production helper TWICE with the same post id.
// The second call must return an empty set. That is deterministic -
// no concurrency, no timing, no shared state.
//
// Run: cargo test --test scheduler_claim_test
// Uses a DEDICATED database file per test (never the shared scratch DB,
// and never data/switchpost.db).

use switchpost::auth::middleware::DEFAULT_USER_ID;
use switchpost::db;
use switchpost::db::queries::{claim_posts_for_publish, get_due_posts};
use switchpost::db::SqlitePool;
use uuid::Uuid;

/// A pool over a database file this test owns.
///
/// The race needs two independent CONNECTIONS, so it must be a FILE:
/// `sqlite::memory:` gives each connection its own database, and
/// `create_pool` sets `max_connections(1)`, so one pool cannot produce
/// two connections either.
async fn pool_for(name: &str) -> SqlitePool {
    let path = format!("target/test-data/scheduler_claim_{name}.db");
    let _ = std::fs::remove_file(&path);
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
async fn a_claimed_post_is_never_claimed_again() {
    let pool = pool_for("once").await;
    let (post_id, _) = seed_due_post(&pool).await;

    let first = get_due_posts(&pool, 10).await.expect("first claim");
    assert!(
        first.iter().any(|p| p.id == post_id),
        "the first claim must take the post"
    );

    // The SECOND claim must not find it. Before the fix it did - and
    // the post was published twice.
    let second = get_due_posts(&pool, 10).await.expect("second claim");
    assert!(
        !second.iter().any(|p| p.id == post_id),
        "a post that is already publishing must not be claimed again"
    );
}

#[tokio::test]
async fn the_claim_helper_is_a_compare_and_set() {
    let pool = pool_for("cas").await;
    let (post_id, _) = seed_due_post(&pool).await;

    // The PRODUCTION helper, called twice with the same id. The second
    // call must return nothing - the row is no longer 'queued'.
    //
    // This is deterministic: it is the same statement, twice, in order.
    let mut tx = pool.begin().await.expect("tx");
    let first = claim_posts_for_publish(&mut *tx, &[post_id])
        .await
        .expect("first claim");
    assert_eq!(first, vec![post_id], "the first claim must take the post");
    tx.commit().await.expect("commit");

    let mut tx2 = pool.begin().await.expect("tx2");
    let second = claim_posts_for_publish(&mut *tx2, &[post_id])
        .await
        .expect("second claim");
    tx2.commit().await.expect("commit 2");
    assert!(
        second.is_empty(),
        "the second claim must get nothing: {second:?}"
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
    let pool = pool_for("later").await;
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
