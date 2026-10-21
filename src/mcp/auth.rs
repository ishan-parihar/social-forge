// ─── Shared MCP Auth Utilities ─────────────────────────────────
// Single-user mode: every MCP/CLI call operates on `DEFAULT_USER_ID`.
// No DB lookup, no JWT validation — the stdio/CLI transport is local
// (shell access already implies trust). The WebUI is the only place
// that needs the password gate, and that's enforced by axum middleware.

use crate::api::AppState;
use crate::auth::middleware::DEFAULT_USER_ID;
use uuid::Uuid;

/// Resolve the local user.
///
/// In single-user mode the answer is `DEFAULT_USER_ID` - and the
/// app's own startup (`ensure_local_user`) creates that row, so on any
/// instance the app has started, the constant IS the local user and the
/// first query below confirms it with no further lookup. But an
/// instance seeded EXTERNALLY (a copy, a fixture, a migrated database)
/// can carry a users row under a DIFFERENT id and no DEFAULT_USER_ID
/// row at all - and then every one of the ~80 call sites reads a user
/// that owns no posts: empty lists, zero streaks, tools that answer
/// confidently and wrongly.
///
/// So: the constant when its row exists (the normal install), otherwise
/// the SINGLE user the instance actually has. An instance with neither
/// is mis-seeded and the error names that instead of a wrong answer.
pub async fn resolve_first_user(state: &AppState) -> Result<Uuid, String> {
    let constant_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)")
            .bind(DEFAULT_USER_ID)
            .fetch_one(&state.db)
            .await
            .map_err(|e| format!("DB error resolving local user: {e}"))?;
    if constant_exists {
        return Ok(DEFAULT_USER_ID);
    }
    // The instance was not seeded by the app: fall back to its single
    // real user, if it has exactly one. Multiple users is not a shape
    // single-user mode defines; refuse rather than pick arbitrarily.
    let seeded: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users ORDER BY created_at LIMIT 2")
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("DB error resolving local user: {e}"))?;
    if seeded.len() == 1 {
        return Ok(seeded[0].0);
    }
    Err(
        "no DEFAULT_USER_ID row and the instance has zero or multiple users - \
         the database is not a valid single-user install".to_string(),
    )
}
