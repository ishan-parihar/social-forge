// ─── Database Pool ─────────────────────────────────────────────
// Creates and manages a sqlx PgPool with connection migration.
//
// Pool sizing is env-configurable so operators can tune for their
// workload and DB instance class:
//   DB_MAX_CONNECTIONS  (default 20)  — max simultaneous DB connections
//   DB_ACQUIRE_TIMEOUT  (default 5)   — seconds to wait for a connection
//   DB_MAX_LIFETIME     (default 3600)— seconds before a conn is recycled
//   DB_IDLE_TIMEOUT     (default 600) — seconds before an idle conn is closed
//
// Without these knobs the pool used sqlx's defaults (10 conns, 30s
// acquire, no max_lifetime) which caused two production issues:
//   (a) under load, requests blocked on acquire_timeout because 10
//       conns wasn't enough for the scheduler + RSS + feed + HTTP
//       workers all competing for connections;
//   (b) after a postgres restart, stale connections weren't recycled
//       (no max_lifetime) so the first query on each one failed.

pub use sqlx::PgPool;

pub mod models;
pub mod queries;

/// Create a connection pool and run migrations.
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let max_connections: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let acquire_timeout_secs: u64 = std::env::var("DB_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let max_lifetime_secs: u64 = std::env::var("DB_MAX_LIFETIME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3600);
    let idle_timeout_secs: u64 = std::env::var("DB_IDLE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(acquire_timeout_secs))
        .max_lifetime(std::time::Duration::from_secs(max_lifetime_secs))
        .idle_timeout(std::time::Duration::from_secs(idle_timeout_secs))
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!(
        "Database connected — pool: max={max_connections}, acquire_timeout={acquire_timeout_secs}s, \
         max_lifetime={max_lifetime_secs}s, idle_timeout={idle_timeout_secs}s. Migrations applied."
    );
    Ok(pool)
}

/// Ensure the single local user row exists. Social Forge is a
/// single-user app — every post, integration, and notification is
/// owned by `DEFAULT_USER_ID`. The `users` row is required only to
/// satisfy foreign-key constraints; the `password` column is unused
/// (auth is via `APP_PASSWORD` env var + signed session cookie, not
/// the DB), so we store a random invalid hash to make that explicit.
pub async fn ensure_local_user(pool: &PgPool) -> anyhow::Result<()> {
    let id = crate::auth::middleware::DEFAULT_USER_ID;
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    if !exists {
        // Random invalid hash — DB password is never checked. The
        // `argon2` prefix keeps the NOT NULL column happy without
        // implying the row is usable for password login.
        let placeholder_hash =
            "$argon2id$v=19$m=19456,t=2,p=1$cmFuZG9tc2FsdA$invalidplaceholderhash";
        sqlx::query(
            "INSERT INTO users (id, email, password, name) VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind("local@socialforge")
        .bind(placeholder_hash)
        .bind("Local User")
        .execute(pool)
        .await?;
        tracing::info!("Created local user row: {id}");
    }
    Ok(())
}
