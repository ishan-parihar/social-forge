// ─── Scheduler leader election tests ──────────────────────────
// Requires: a running Postgres at DATABASE_URL.
//
// The lease primitive is the whole point of the feature, so these tests
// exercise the real Postgres advisory lock rather than a mock: the claim
// under test ("N replicas against one DB elect exactly one poller") is a
// claim about database behavior, and a mock would only prove the mock.
//
// Skips (rather than fails) when no database is reachable, so a machine
// without Postgres reports a pass with a printed skip note.

use sqlx::PgPool;

/// All tests contend for the one real lease key and `cargo test` runs
/// them on parallel threads. Serialise them so one test's held lease
/// can't be mistaken for another test's.
static TEST_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Pool with room for several sessions, or `None` when no database is
/// reachable.
async fn test_pool() -> Option<PgPool> {
    social_forge::config::load_dotenv();
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()?;
    Some(pool)
}

/// The core guarantee: two instances racing for the same database elect
/// exactly one leader. Every `try_acquire_leader` call takes its own
/// pooled connection, so this is two real Postgres sessions contending for
/// one session-level advisory lock.
#[tokio::test]
async fn two_instances_elect_exactly_one_leader() {
    let _gate = TEST_GATE.lock().await;
    let Some(pool) = test_pool().await else {
        eprintln!("skip: no DATABASE_URL / unreachable DB");
        return;
    };

    let first = social_forge::lease::try_acquire_leader(&pool)
        .await
        .expect("acquire #1");
    let second = social_forge::lease::try_acquire_leader(&pool)
        .await
        .expect("acquire #2");

    assert!(first.is_some(), "first instance must win the lease");
    assert!(
        second.is_none(),
        "second instance must be a follower while the lease is held"
    );

    // A third replica joining an already-leader cluster is also a follower.
    let third = social_forge::lease::try_acquire_leader(&pool)
        .await
        .expect("acquire #3");
    assert!(third.is_none(), "third instance must be a follower");

    // Drop the holder's lease so a following test starts clean regardless of
    // the order these locals go out of scope.
    first.unwrap().release().await;
    drop(second);
    drop(third);
}

/// A released lease is immediately available to the next instance — this is
/// what makes a leader restart hand off with no operator action, and what
/// makes a single instance behave exactly as it did before election existed.
#[tokio::test]
async fn released_lease_is_reacquired_by_next_instance() {
    let _gate = TEST_GATE.lock().await;
    let Some(pool) = test_pool().await else {
        eprintln!("skip: no DATABASE_URL / unreachable DB");
        return;
    };

    let leader = social_forge::lease::try_acquire_leader(&pool)
        .await
        .expect("acquire leader")
        .expect("a lone instance must win");
    leader.release().await;

    let successor = social_forge::lease::try_acquire_leader(&pool)
        .await
        .expect("acquire successor");
    assert!(
        successor.is_some(),
        "successor must win immediately after release"
    );
}

/// Single-instance behavior is unchanged: the first and only instance always
/// wins, every time.
#[tokio::test]
async fn lone_instance_always_wins_the_lease() {
    let _gate = TEST_GATE.lock().await;
    let Some(pool) = test_pool().await else {
        eprintln!("skip: no DATABASE_URL / unreachable DB");
        return;
    };

    for round in 1..=3 {
        let lease = social_forge::lease::try_acquire_leader(&pool)
            .await
            .unwrap_or_else(|e| panic!("round {round}: acquire failed: {e}"))
            .unwrap_or_else(|| panic!("round {round}: lone instance must win the lease"));
        lease.release().await;
    }
}
