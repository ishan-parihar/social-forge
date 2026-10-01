// ─── Database Pool ─────────────────────────────────────────────
// Creates and manages a sqlx SqlitePool with connection migration.
//
// SQLite is a single-writer store, so the pool is deliberately sized to
// one connection: more writers would only serialize on the file lock,
// and WAL still lets readers run concurrently with the one writer.
// Migrations run once on boot via `sqlx::migrate!`.
//
// Connection tuning is done through `SqliteConnectOptions` (the builder),
// never URL query params:
//   journal_mode = WAL         — readers don't block the writer
//   synchronous  = NORMAL      — durable enough under WAL, much faster
//   busy_timeout = 5s          — wait instead of immediately SQLITE_BUSY
//   foreign_keys = ON          — SQLite defaults this OFF per connection

use std::str::FromStr;

pub use sqlx::SqlitePool;

pub mod models;
pub mod queries;
pub mod types;

/// Create a connection pool and run migrations.
pub async fn create_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    let options = sqlx::sqlite::SqliteConnectOptions::from_str(database_url)
        .map_err(|e| anyhow::anyhow!("Invalid DATABASE_URL `{database_url}`: {e}"))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .foreign_keys(true);

    // sqlx does not create the parent directory for a file-backed SQLite
    // database, and `data/` is gitignored — so the default DATABASE_URL
    // needs this to work on a fresh clone.
    if let Some(parent) = options.get_filename().parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database connected — sqlite pool: max_connections=1, WAL. Migrations applied.");
    Ok(pool)
}

/// Ensure the single local user row exists. Social Forge is a
/// single-user app — every post, integration, and notification is
/// owned by `DEFAULT_USER_ID`. The `users` row is required only to
/// satisfy foreign-key constraints; the `password` column is unused
/// (auth is via `APP_PASSWORD` env var + signed session cookie, not
/// the DB), so we store a random invalid hash to make that explicit.
pub async fn ensure_local_user(pool: &SqlitePool) -> anyhow::Result<()> {
    let id = crate::auth::middleware::DEFAULT_USER_ID;
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)")
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
            "INSERT INTO users (id, email, password, name) VALUES (?, ?, ?, ?)
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