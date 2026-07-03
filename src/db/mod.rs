// ─── Database Pool ─────────────────────────────────────────────
// Creates and manages a sqlx PgPool with connection migration.

pub use sqlx::PgPool;

pub mod models;
pub mod queries;

/// Create a connection pool and run migrations
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database connected and migrations applied");
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
