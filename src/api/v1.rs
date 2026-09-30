// ─── Public API v1 ─────────────────────────────────────────────
// The stable, versioned surface for programmatic clients
// (`Authorization: Bearer sf_…`). Single-user scope only — same
// `AuthenticatedUser` resolution as the dashboard, no per-key
// permissions.
//
// Deliberately thin: every handler delegates to the existing
// implementation, so v1 responses can never drift from `/api/*`.
// Read-only (GET) — writes stay behind the cookie-authenticated
// dashboard until a versioned write contract exists.

use axum::{routing::get, Router};

use super::{auth, integrations, posts, AppState};

/// Routes mounted at the prefix. Merged into the protected router, so
/// the auth and CSRF middleware layers apply as they do elsewhere.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/me", get(auth::me))
        .route("/api/v1/posts", get(posts::list))
        .route("/api/v1/integrations", get(integrations::list))
}
