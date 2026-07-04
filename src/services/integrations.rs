// ─── Integration Service ──────────────────────────────────────
// Shared business logic for OAuth flow and integration management.
// Used by both `api/integrations.rs` (HTTP) and `mcp/tools_integrations.rs` (MCP).

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::models::Integration;
use crate::db::queries;
use crate::realtime::Broadcaster;
use crate::social::common;
use crate::social::registry::ProviderRegistry;

/// Result type for service operations
pub type ServiceResult<T> = Result<T, String>;

/// OAuth initiate result
pub struct OAuthInitiateResult {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
}

/// Integration service
pub struct IntegrationService;

impl IntegrationService {
    /// Initiate OAuth flow for a provider
    pub async fn initiate_connect(
        db: &PgPool,
        registry: &ProviderRegistry,
        user_id: Uuid,
        provider_identifier: &str,
        app_url: &str,
    ) -> ServiceResult<OAuthInitiateResult> {
        let provider = registry
            .get(provider_identifier)
            .ok_or_else(|| format!("Unknown provider: {provider_identifier}"))?;

        let state = common::generate_state();
        let code_verifier = common::generate_code_verifier();

        let redirect_uri = if provider.uses_oauth() {
            format!("{app_url}/api/auth/callback")
        } else {
            format!("{app_url}/api/auth/connect")
        };

        let auth_response = provider
            .generate_auth_url(&state, &code_verifier, &redirect_uri)
            .await
            .map_err(|e| format!("Failed to generate auth URL: {e}"))?;

        // Save OAuth state
        queries::save_oauth_state(
            db,
            &state,
            provider_identifier,
            &code_verifier,
            Some(&format!("{user_id}:{redirect_uri}")),
        )
        .await
        .map_err(|e| format!("Database error: {e}"))?;

        Ok(OAuthInitiateResult {
            auth_url: auth_response.url,
            state,
            code_verifier,
        })
    }

    /// Complete OAuth exchange from a callback state + code.
    /// Extracts provider_identifier and user_id from the stored OAuth state.
    /// Encrypts tokens at rest when token_key is configured.
    pub async fn complete_connect(
        db: &PgPool,
        registry: &ProviderRegistry,
        broadcaster: &Broadcaster,
        state: &str,
        code: &str,
        token_key: Option<&[u8; 32]>,
    ) -> ServiceResult<Integration> {
        // Verify state and get stored verifier
        let oauth_state = queries::get_oauth_state(db, state)
            .await
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Invalid or expired OAuth state".to_string())?;

        let provider_identifier = &oauth_state.provider;

        let provider = registry
            .get(provider_identifier)
            .ok_or_else(|| format!("Unknown provider: {provider_identifier}"))?;

        // Parse user_id and redirect_uri from stored redirect_uri
        let stored_uri = oauth_state
            .redirect_uri
            .as_deref()
            .unwrap_or("");
        let parts: Vec<&str> = stored_uri.split(':').collect();
        let user_id = Uuid::parse_str(parts.first().ok_or("Invalid state format")?)
            .map_err(|e| format!("Invalid user ID: {e}"))?;
        // Reconstruct redirect_uri (UUID is first part, rest is the URL which may contain colons)
        let redirect_uri = if parts.len() > 1 {
            parts[1..].join(":")
        } else {
            String::new()
        };

        let token = provider
            .exchange_code(code, &oauth_state.code_verifier, &redirect_uri)
            .await
            .map_err(|e| format!("Token exchange failed: {e}"))?;

        // Delete the used state
        if let Err(e) = queries::delete_oauth_state(db, state).await {
            tracing::warn!("DB operation failed: {e}");
        }

        // Encrypt tokens before storing if encryption key is configured
        let enc_access_token = match token_key {
            Some(key) => crate::crypto::encrypt_string(&token.access_token, key)
                .unwrap_or_else(|_| token.access_token.clone()),
            None => token.access_token.clone(),
        };
        let enc_refresh_token = token.refresh_token.as_ref().and_then(|rt| {
            token_key.and_then(|key| {
                crate::crypto::encrypt_string(rt, key).ok()
            })
        });

        // Create or update integration
        let integration = queries::create_integration(
            db,
            user_id,
            provider_identifier,
            provider.name(),
            &token.provider_user_id,
            &enc_access_token,
            enc_refresh_token.as_deref().or(token.refresh_token.as_deref()),
            token.expires_in.map(|exp| {
                Utc::now() + chrono::Duration::seconds(exp as i64)
            }),
            Some(&token.name),
            token.picture.as_deref(),
            None,
            None, // root_internal_id — set by pages API for sub-accounts
        None, // auth_method
        )
        .await
        .map_err(|e| format!("Database error: {e}"))?;

        broadcaster.send(
            "integration_connected",
            &serde_json::json!({
                "id": integration.id.to_string(),
                "provider": provider_identifier,
            }),
        );

        Ok(integration)
    }

    /// List all integrations for a user
    pub async fn list(
        db: &PgPool,
        user_id: Uuid,
    ) -> ServiceResult<Vec<Integration>> {
        queries::list_integrations(db, user_id)
            .await
            .map_err(|e| format!("Database error: {e}"))
    }

    /// Disconnect (delete) an integration
    pub async fn disconnect(
        db: &PgPool,
        broadcaster: &Broadcaster,
        user_id: Uuid,
        integration_id: Uuid,
    ) -> ServiceResult<bool> {
        let deleted = queries::delete_integration(db, integration_id, user_id)
            .await
            .map_err(|e| format!("Database error: {e}"))?;

        if deleted {
            broadcaster.send(
                "integration_disconnected",
                &serde_json::json!({"id": integration_id.to_string()}),
            );
        }

        Ok(deleted)
    }
}
