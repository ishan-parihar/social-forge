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
