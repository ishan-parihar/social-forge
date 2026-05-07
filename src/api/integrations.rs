// ─── Integrations API Routes ──────────────────────────────────
// Social channel connections: OAuth flow initiation, callback, listing, deletion.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::models::IntegrationPublic;
use crate::db::queries;
use crate::error::AppError;

use super::AppState;

// ── Request/Response Types ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectResponse {
    pub url: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct IntegrationListResponse {
    pub integrations: Vec<IntegrationPublic>,
}



// ── Handlers ─────────────────────────────────────────────────

/// GET /api/integrations — list all connected channels
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<IntegrationListResponse>, AppError> {
    let integrations = queries::list_integrations(&state.db, auth.user_id).await?;
    let public: Vec<IntegrationPublic> = integrations.into_iter().map(Into::into).collect();
    Ok(Json(IntegrationListResponse { integrations: public }))
}

/// GET /api/integrations/connect/:provider — initiate OAuth flow
pub async fn connect(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(provider): Path<String>,
    Query(query): Query<ConnectQuery>,
) -> Result<Json<ConnectResponse>, AppError> {
    // Check credentials first
    if state.config.provider_credentials(&provider).is_none() {
        return Err(AppError::BadRequest(
            format!("Provider '{provider}' is not configured. Set the corresponding environment variables first.")
        ));
    }

    // Find the provider in the registry
    let provider_obj = state
        .providers
        .get(&provider)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider: {provider}")))?;

    // Handle non-OAuth providers (e.g., Bluesky with app passwords)
    if !provider_obj.uses_oauth() {
        // For non-OAuth providers, call exchange code directly to auto-connect
        let redirect_uri = query
            .redirect_uri
            .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

        let token = provider_obj
            .exchange_code("", "", &redirect_uri)
            .await
            .map_err(|e| {
                tracing::error!("Direct connect failed for {}: {e}", provider);
                AppError::Provider(format!("Failed to connect {provider}: {e}. Check your env vars."))
            })?;

        queries::create_integration(
            &state.db,
            auth.user_id,
            &provider,
            &token.name,
            &token.provider_user_id,
            &token.access_token,
            token.refresh_token.as_deref(),
            token.expires_in.map(|exp| {
                chrono::Utc::now() + chrono::Duration::seconds(exp as i64)
            }),
            Some(&token.name),
            token.picture.as_deref(),
            None,
        )
        .await?;

        state.broadcast.send(
            "integration_connected",
            &serde_json::json!({
                "provider": provider,
                "name": token.name,
            }),
        );

        return Ok(Json(ConnectResponse {
            url: format!("Connected to {} as {}. No browser OAuth needed.", provider, token.name),
            state: "auto".into(),
        }));
    }

    // Generate PKCE and OAuth URL
    let code_verifier = crate::social::common::generate_code_verifier();
    let oauth_state = crate::social::common::generate_state();
    let redirect_uri = query
        .redirect_uri
        .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

    let auth_url = provider_obj
        .generate_auth_url(&oauth_state, &code_verifier, &redirect_uri)
        .await
        .map_err(|e| AppError::Provider(format!("Failed to generate auth URL: {e}")))?;

    // Store OAuth state with user context
    queries::save_oauth_state(
        &state.db,
        &oauth_state,
        &provider,
        &code_verifier,
        Some(&format!("{}:{}", auth.user_id, redirect_uri)),
    )
    .await?;

    Ok(Json(ConnectResponse {
        url: auth_url.url,
        state: oauth_state,
    }))
}

/// GET /api/auth/callback — complete OAuth flow
pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Retrieve stored OAuth state
    let stored = queries::get_oauth_state(&state.db, &query.state)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired OAuth state".into()))?;

    // Parse user_id from redirect_uri (stored as "user_id:redirect_uri")
    let user_id_str = stored
        .redirect_uri
        .as_ref()
        .and_then(|r| r.split(':').next())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::BadRequest("Invalid OAuth state data".into()))?;

    // Exchange code for token
    let provider_obj = state
        .providers
        .get(&stored.provider)
        .ok_or_else(|| AppError::BadRequest("Provider not found".into()))?;

    let token = provider_obj
        .exchange_code(&query.code, &stored.code_verifier, "")
        .await
        .map_err(|e| {
            tracing::error!("Token exchange failed for {}: {e}", stored.provider);
            AppError::Provider("Failed to exchange authorization code".into())
        })?;

    // Save integration
    queries::create_integration(
        &state.db,
        user_id_str,
        &stored.provider,
        &token.name,
        &token.provider_user_id,
        &token.access_token,
        token.refresh_token.as_deref(),
        token.expires_in.map(|exp| {
            chrono::Utc::now() + chrono::Duration::seconds(exp as i64)
        }),
        Some(&token.name),
        token.picture.as_deref(),
        None,
    )
    .await?;

    // Clean up OAuth state
    queries::delete_oauth_state(&state.db, &query.state).await?;

    tracing::info!(
        "Integration connected: {} ({}) for user {}",
        stored.provider,
        token.name,
        user_id_str
    );

    // Notify via broadcast
    state.broadcast.send(
        "integration_connected",
        &serde_json::json!({
            "provider": stored.provider,
            "name": token.name,
        }),
    );

    // Redirect to frontend
    let frontend_url = state.config.frontend_url.clone();
    Ok(Json(serde_json::json!({
        "success": true,
        "provider": stored.provider,
        "name": token.name,
        "redirect": format!("{}/channels", frontend_url),
    })))
}

/// DELETE /api/integrations/:id — remove a connected channel
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Delete posts for this integration first
    let deleted = queries::delete_integration(&state.db, id, auth.user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Integration not found".into()));
    }

    state.broadcast.send(
        "integration_disconnected",
        &serde_json::json!({"id": id}),
    );

    Ok(Json(serde_json::json!({"deleted": true})))
}
// rebuild
