// ─── Shared MCP Auth Utilities ─────────────────────────────────
// Single-user mode: every MCP/CLI call operates on `DEFAULT_USER_ID`.
// No DB lookup, no JWT validation — the stdio/CLI transport is local
// (shell access already implies trust). The WebUI is the only place
// that needs the password gate, and that's enforced by axum middleware.

use uuid::Uuid;
use crate::api::AppState;
use crate::auth::middleware::DEFAULT_USER_ID;

/// Resolve the local user. In single-user mode this is always
/// `DEFAULT_USER_ID` — no DB lookup needed. The function signature
/// is kept for compatibility with the ~80 existing call sites.
pub(crate) async fn resolve_first_user(_state: &AppState) -> Result<Uuid, String> {
    Ok(DEFAULT_USER_ID)
}
