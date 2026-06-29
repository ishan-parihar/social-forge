// ─── Shared MCP Auth Utilities ─────────────────────────────────
// Common authentication helpers used across all MCP tool modules.
// Single source of truth for user resolution and token decryption.

use uuid::Uuid;
use crate::api::AppState;

/// Resolve the first registered user.
/// Prefers a user with integrations, falls back to any user.
pub(crate) async fn resolve_first_user(state: &AppState) -> Result<Uuid, String> {
    let user = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM users u \
         WHERE EXISTS (SELECT 1 FROM integrations i WHERE i.user_id = u.id) \
         LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(id) = user {
        return Ok(id);
    }

    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users LIMIT 1")
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No user registered. Use auth.register first.".to_string())
}

/// Decrypt a token from the database.
/// Attempts base64 decoding first, falls back to raw string.
/// For encrypted tokens, callers should use crypto::decrypt_string with token_key instead.
pub(crate) fn decrypt_token(encrypted: &str) -> String {
    if let Ok(decoded) = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted,
    ) {
        String::from_utf8(decoded).unwrap_or_else(|_| encrypted.to_string())
    } else {
        encrypted.to_string()
    }
}
