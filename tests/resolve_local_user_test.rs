// ─── resolve_first_user: the seeded-instance fix ───
//
// `resolve_first_user` behind ~80 MCP/CLI call sites used to return the
// hard-coded DEFAULT_USER_ID unconditionally. On any instance the app
// itself started, `ensure_local_user` has created that row, so the
// constant was right. But an EXTERNALLY-seeded instance (a copy, a
// fixture, a migrated database) can carry its single users row under a
// different id - and then every tool read a user that owned nothing:
// empty lists, zero streaks, confident wrong answers. This session's
// own seeded audit instance is exactly that shape.
//
// The fix resolves: the constant when its row exists, otherwise the
// instance's single real user, otherwise a loud error. These tests pin
// all three arms against the real helper.
//
// Run: cargo test --test resolve_local_user_test

use switchpost::api::AppState;
use switchpost::auth::middleware::DEFAULT_USER_ID;
use switchpost::config::Config;
use switchpost::db;
use switchpost::mcp::auth::resolve_first_user;
use switchpost::realtime::Broadcaster;
use switchpost::social::registry::ProviderRegistry;
use uuid::Uuid;

/// A state on THIS TEST's own database, WITHOUT ensure_local_user: the
/// users table is whatever this test makes it.
///
/// The tests in this binary run in PARALLEL and each one rewrites the
/// `users` table — one seeds a row while another asserts the table is
/// empty. A shared per-binary file made them race, so the empty-instance
/// arm saw the other test's seeded row and resolved it instead of
/// refusing. Each test gets its own file (same pattern as
/// `config::test_database_url`, one level finer).
async fn bare_state(name: &str) -> AppState {
    switchpost::config::load_dotenv_for_tests();
    let mut config = Config::from_env().expect("config");
    let base = config.database_url.trim_end_matches(".db").to_string();
    config.database_url = format!("{base}-{name}.db");
    let pool = db::create_pool(&config.database_url).await.expect("pool");
    let registry = ProviderRegistry::new(&config, None, None);
    AppState {
        db: pool,
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

async fn user_count(state: &AppState) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap()
}

/// The normal install: DEFAULT_USER_ID's row exists, the answer is the
/// constant, no extra lookup.
#[tokio::test]
async fn the_constant_wins_when_its_row_exists() {
    let state = bare_state("constant").await;
    db::ensure_local_user(&state.db).await.expect("local user");
    let resolved = resolve_first_user(&state).await.expect("resolve");
    assert_eq!(resolved, DEFAULT_USER_ID);
}

/// The externally-seeded instance: no constant row, one real user under
/// a different id - the tools must read THAT user, not a ghost. This
/// is the exact shape of the seeded audit instance that showed every
/// MCP tool answering 0 for the founder's real posts.
#[tokio::test]
async fn an_externally_seeded_instance_resolves_its_real_user() {
    let state = bare_state("seeded").await;
    // Clean slate: this binary's other tests may have created users.
    sqlx::query("DELETE FROM users")
        .execute(&state.db)
        .await
        .expect("clear users");
    let seeded = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Seeded')")
        .bind(seeded)
        .bind(format!("{seeded}@seed.test"))
        .execute(&state.db)
        .await
        .expect("seed user");
    assert_eq!(user_count(&state).await, 1, "exactly one user, not the constant");

    let resolved = resolve_first_user(&state).await.expect("resolve");
    assert_eq!(
        resolved, seeded,
        "no DEFAULT_USER_ID row: the instance's single real user is the answer"
    );
}

/// A mis-seeded instance (no users at all) is a loud error, not a
/// confident wrong answer.
#[tokio::test]
async fn an_empty_instance_is_a_loud_error() {
    let state = bare_state("empty").await;
    sqlx::query("DELETE FROM users")
        .execute(&state.db)
        .await
        .expect("clear users");
    let err = resolve_first_user(&state).await.expect_err("must refuse");
    assert!(
        err.contains("not a valid single-user install"),
        "the error must name the mis-seeding: {err}"
    );
}

/// The refused arm: an instance with TWO users and no DEFAULT_USER_ID
/// row is not a shape single-user mode defines - the error names that
/// rather than picking one arbitrarily. This arm was implemented and
/// promised by the doc comment but never executed before this test:
/// the seeded instance has one user, the normal install one, so both
/// live runs took the other two arms (9.38: a branch that has never
/// run must say so - now it has run).
#[tokio::test]
async fn two_users_without_the_constant_is_a_loud_refusal() {
    let state = bare_state("two").await;
    sqlx::query("DELETE FROM users")
        .execute(&state.db)
        .await
        .expect("clear users");
    for n in 0..2 {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password, name) VALUES (?, ?, 'x', 'Two')")
            .bind(id)
            .bind(format!("{id}@two{n}.test"))
            .execute(&state.db)
            .await
            .expect("seed user");
    }
    let err = resolve_first_user(&state).await.expect_err("must refuse");
    assert!(
        err.contains("zero or multiple users"),
        "the error must name the multiple-user shape: {err}"
    );
}
