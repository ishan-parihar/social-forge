// ─── Scheduler Leader Election ─────────────────────────────────
//
// N replicas of the single binary may point at one Postgres. Only one
// of them may run the six background pollers (scheduler, RSS, feed,
// analytics cache, streak reset, plug runner) — otherwise six replicas
// mean six concurrent publishes and six times the platform API traffic
// for work that is by definition once-only.
//
// The election is a single Postgres **session-level advisory lock**,
// not a `scheduler_leases` table. Reasoning:
//
//   * A session advisory lock is released by Postgres itself the moment
//     the holding session ends. A crashed or SIGKILLed replica frees it
//     immediately, so nothing has to be reaped on restart — which is the
//     property a heartbeat table exists to approximate.
//   * No heartbeat task, no `expires_at`, no expiry sweep, no clock
//     comparison between replicas, and no migration. The lease cannot
//     go stale, which is the failure mode a table lease has to defend
//     against and the reason it needs a migration plus a reaper.
//
// The one thing it costs: the lock lives on a *connection*, so it must
// be pinned by holding that connection for the process lifetime. The
// returned `LeaderLease` owns it; dropping the lease (or the process)
// drops the connection and frees the lock.
//
// Deliberate simplification: if the pinned connection dies while the
// process is still alive (Postgres restart, network blip), Postgres
// frees the lock and another replica may win it, but this process keeps
// polling until something restarts it — the pollers are fire-and-forget
// `tokio::spawn`s that return no handle, so re-electing would mean
// reworking the loop bodies, not this file.
// ponytail: leader loss is detected, not recovered — handle-less pollers
// would need spawn/join plumbing to re-elect. Add a re-election task if
// a pinned-connection loss ever matters in practice.

use anyhow::Context;

use sqlx::pool::PoolConnection;
use sqlx::PgPool;

/// Advisory-lock key for the scheduler lease. Arbitrary but stable: any
/// other application sharing this database picks a different key, so the
/// two never contend.
const LEASE_LOCK_KEY: i64 = 8_804_221_001;

/// A held scheduler lease. While this value is alive, this process is the
/// leader and may run the background pollers. Dropping it releases the
/// advisory lock (the owned connection goes back to the pool / closes).
pub struct LeaderLease {
    /// Pinned connection: the session-level advisory lock is a property of
    /// this session, so it must be held, not borrowed for one query. Keeping
    /// the `PoolConnection` in the struct (rather than the `PgPool`) is what
    /// stops the pool from recycling the session out from under the lock.
    _conn: PoolConnection<sqlx::Postgres>,
    /// Instance identity, for the startup log line only.
    holder: String,
}

impl LeaderLease {
    /// Instance identifier recorded in the leadership log line — hostname
    /// and pid, so an operator reading logs can tell replicas apart.
    pub fn holder(&self) -> &str {
        &self.holder
    }
}

/// Try to become the scheduler leader.
///
/// Returns `Ok(Some(lease))` when this process won the election and must
/// run the pollers, `Ok(None)` when another replica already holds the lease
/// and this process should serve API/MCP only.
///
/// A database error resolves to `Ok(Some(lease))` — i.e. fail open. Reaching
/// this function means `db::create_pool` already connected and ran migrations,
/// so a failure here is a connection that died in the intervening
/// milliseconds; refusing to elect would silently stop a lone instance from
/// publishing scheduled posts, which is a worse break than the brief
/// duplicate-poll window a six-way transient failure would cause.
pub async fn try_acquire_leader(pool: &PgPool) -> anyhow::Result<Option<LeaderLease>> {
    let holder = instance_id();

    // Two distinct connections: the lock is re-entrant within a session, so
    // reusing one session would let a second acquisition report success.
    let mut conn = pool
        .acquire()
        .await
        .context("Failed to acquire connection for scheduler lease")?;

    let won: bool = match sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
        .bind(LEASE_LOCK_KEY)
        .fetch_one(&mut *conn)
        .await
    {
        Ok(won) => won,
        Err(e) => {
            tracing::error!(
                "Scheduler lease query failed ({e}) — starting pollers anyway (fail-open). \
                 A lone instance keeps publishing; a multi-replica fleet may double-poll \
                 until restart."
            );
            return Ok(Some(LeaderLease { _conn: conn, holder }));
        }
    };

    if won {
        tracing::info!("Scheduler lease acquired — this instance runs the background pollers");
        Ok(Some(LeaderLease { _conn: conn, holder }))
    } else {
        tracing::info!(
            "Scheduler lease held by another replica — this instance serves API/MCP only \
             (background pollers not started)"
        );
        Ok(None)
    }
}

/// Explicitly release the lease. Not required for correctness (dropping the
/// connection releases the advisory lock), but it makes the handover visible
/// in the log and lets a replica stand down deterministically.
impl LeaderLease {
    pub async fn release(mut self) {
        let holder = self.holder.clone();
        if let Err(e) =
            sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock($1)")
                .bind(LEASE_LOCK_KEY)
                .fetch_one(&mut *self._conn)
                .await
        {
            tracing::warn!("Failed to release scheduler lease: {e}");
        } else {
            tracing::info!("Scheduler lease released by {holder}");
        }
        // `self._conn` drops here, freeing the advisory lock either way.
    }
}

/// Hostname + pid, falling back to something stable if the hostname is
/// unavailable. Log-only — never a correctness input.
fn instance_id() -> String {
    let host = std::env::var("HOSTNAME")
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown-host".into());
    format!("{host}/{}", std::process::id())
}
