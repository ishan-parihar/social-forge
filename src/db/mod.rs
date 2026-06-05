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

/// Ensure the DEFAULT_USER_ID user exists in the database.
/// This prevents foreign key violations when OAuth callbacks or
/// the Telegram bot token flow try to create integrations for
/// the hardcoded default user.
pub async fn ensure_default_user(pool: &PgPool) -> anyhow::Result<()> {
    let id = crate::auth::middleware::DEFAULT_USER_ID;
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    if !exists {
        let hash = crate::auth::jwt::hash_password("socialforge")
            .map_err(|e| anyhow::anyhow!("Failed to hash default user password: {e}"))?;
        sqlx::query(
            "INSERT INTO users (id, email, password, name) VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind("default@socialforge.local")
        .bind(&hash)
        .bind("Default User")
        .execute(pool)
        .await?;
        tracing::info!("Created default user: {id}");
    }
    Ok(())
}
