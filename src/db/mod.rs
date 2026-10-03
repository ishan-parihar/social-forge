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
//   busy_timeout = 15s         — wait instead of immediately SQLITE_BUSY
//   foreign_keys = ON          — SQLite defaults this OFF per connection
//
// SINGLE-WRITER DISCIPLINE (G-08). Two distinct layers matter:
//
//  1. IN-PROCESS — `max_connections(1)`. One connection, so no two statements
//     in this process can race for the write lock.
//  2. CROSS-PROCESS — `busy_timeout`. Every process (serve, mcp, and each
//     short-lived CLI invocation) opens its own pool against the same file, so
//     `max_connections(1)` does nothing across them; the 15s timeout is what
//     makes a concurrent writer WAIT instead of failing with SQLITE_BUSY.
//     The default was 5s, which the publish scheduler could exceed mid-batch
//     (a slow platform write holding the lock) and then drop a queued post.
//     15s covers the longest single publish transaction with headroom.
//
// Because cross-process writers queue rather than fail, run ONE long-lived
// process per database file. Two `social-forge serve` instances on the same
// `DATABASE_URL` will both hold the scheduler and double-publish. `social-forge
// audit` reports the live `journal_mode` / `busy_timeout` read back from the
// open pool, and `WRITER_MODE` records which process owns the file.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

/// How long a writer waits for the file lock before SQLITE_BUSY. Sized above
/// the worst-case single publish transaction (network write + post + analytics)
/// so a queued post is never dropped just because another process held the lock.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(15);

/// Advisory lock file sitting next to the database file. Held with `O_EXCL`
/// semantics by long-lived processes so a SECOND `serve` on the same
/// `DATABASE_URL` fails loudly at boot instead of silently double-publishing.
/// The actual name is `<db-file>.writer.lock` (see `claim_writer_role`).
const WRITER_MODE: &str = "writer.lock";

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
        .busy_timeout(BUSY_TIMEOUT)
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
    tracing::info!(
        "Database connected — sqlite pool: max_connections=1, WAL, busy_timeout={}s. Migrations applied.",
        BUSY_TIMEOUT.as_secs()
    );
    Ok(pool)
}

/// Claim the single-writer role for `database_url`'s file by creating an
/// exclusive lock file next to it.
///
/// Call this ONLY from long-lived processes (`serve`, `mcp`) — the ones that
/// run the publish scheduler. A short-lived CLI read (`providers`, `posts
/// list`) must not take the lock, or it would block the daemon it is querying.
///
/// The path is derived from the PARSED connect options, never from string
/// surgery on the URL: `Path::new("sqlite://data/app.db").parent()` yields the
/// literal `sqlite:/data`, which silently created a bogus lock directory.
/// `SqliteConnectOptions::get_filename()` returns the real file path.
///
/// ponytail: `O_EXCL` + PID in the file, released on process exit. If a stale
/// lock survives a `kill -9`, delete `data/.social-forge-writer.lock` by hand.
/// A PID-liveness probe is the upgrade if stale locks actually show up.
pub fn claim_writer_role(database_url: &str) -> anyhow::Result<File> {
    let options = sqlx::sqlite::SqliteConnectOptions::from_str(database_url)
        .map_err(|e| anyhow::anyhow!("Invalid DATABASE_URL for writer lock: {e}"))?;
    let filename = options.get_filename().to_path_buf();

    // An in-memory database has no file to lock — nothing to coordinate.
    if filename.as_os_str().is_empty() {
        tracing::info!("In-memory SQLite — no writer lock required");
        return Ok(File::open("/dev/null")?);
    }

    let path = PathBuf::from(filename).with_extension(WRITER_MODE);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut f) => {
            use std::io::Write as _;
            writeln!(f, "{}", std::process::id())?;
            tracing::info!("Single-writer lock acquired: {}", path.display());
            Ok(f)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // PID-liveness probe: a lock whose recorded PID is gone is stale
            // (a graceful shutdown does not delete the file, and neither does
            // kill -9). Take it over once; a live owner keeps failing loudly.
            // unwrap_or(true) fails safe — an unreadable lock is treated as held.
            let owner_alive = fs::read_to_string(&path)
                .ok()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .map(|pid| Path::new("/proc").join(pid.to_string()).exists())
                .unwrap_or(true);
            if owner_alive {
                return Err(anyhow::anyhow!(
                    "Another social-forge process already owns the writer lock for this database \
                     ({}). SQLite is single-writer: run ONE long-lived process per \
                     DATABASE_URL, or delete the lock if that process is gone.",
                    path.display()
                ));
            }
            tracing::warn!(
                "Writer lock {} is stale (owner gone) — taking it over",
                path.display()
            );
            fs::remove_file(&path)?;
            let mut f = OpenOptions::new().write(true).create_new(true).open(&path)?;
            use std::io::Write as _;
            writeln!(f, "{}", std::process::id())?;
            tracing::info!("Single-writer lock acquired: {}", path.display());
            Ok(f)
        }
        Err(e) => Err(anyhow::anyhow!("Cannot claim writer lock {}: {e}", path.display())),
    }
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