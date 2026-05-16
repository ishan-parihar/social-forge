// ─── Integrations API Routes ──────────────────────────────────
// Social channel connections: OAuth flow initiation, callback, listing, deletion.

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::jwt;
use crate::auth::middleware::AuthenticatedUser;
use crate::crypto;
use crate::db::models::Integration;
use crate::db::models::IntegrationPublic;
use crate::db::queries;
use crate::error::AppError;
use crate::services::integrations::IntegrationService;
use crate::social::PageInfo;

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

    // Handle non-OAuth providers (e.g., Bluesky, Telegram)
    if !provider_obj.uses_oauth() {
        let redirect_uri = query
            .redirect_uri
            .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

        // For one-time-token providers (Telegram), give instructions
        if provider_obj.one_time_token() {
            // Generate a one-time code that the user will send in the chat
            let code = uuid::Uuid::new_v4().to_string();
            return Ok(Json(ConnectResponse {
                url: format!(
                    "Open Telegram, start a chat with this bot, and send: /connect {}",
                    code
                ),
                state: "one-time-token".into(),
            }));
        }

        let token = provider_obj
            .exchange_code("", "", &redirect_uri)
            .await
            .map_err(|e| {
                tracing::error!("Direct connect failed for {}: {e}", provider);
                AppError::Provider(format!(
                    "Failed to connect {}. Make sure the provider env vars are set. Error: {e}",
                    provider
                ))
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
///
/// For single-step providers: redirects to onboarding page at {app_url}/?connected=...
/// For multi-step providers (isBetweenSteps): redirects to page-picker at {app_url}/?pending=...
pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    let app_url = state.config.app_url.clone();

    match IntegrationService::complete_connect(
        &state.db,
        &state.providers,
        &state.broadcast,
        &query.state,
        &query.code,
        state.token_key.as_ref(),
    )
    .await
    {
        Ok(integration) => {
            tracing::info!(
                "Integration connected: {} ({}) for user {}",
                integration.provider_identifier,
                integration.provider_name,
                integration.user_id
            );

            // Multi-step providers (isBetweenSteps) redirect to page-picker
            if let Some(provider_obj) = state.providers.get(&integration.provider_identifier) {
                if provider_obj.is_between_steps() {
                    let token = jwt::create_token(integration.user_id, &state.config.jwt_secret)
                        .map_err(|e| AppError::Internal(format!("JWT creation: {e}")))?;
                    return Ok(Redirect::to(&format!(
                        "{}/?pending={}&integration_id={}&token={}",
                        app_url,
                        integration.provider_identifier,
                        integration.id,
                        token,
                    )));
                }
            }

            Ok(Redirect::to(&format!(
                "{}/?connected={}&name={}",
                app_url,
                integration.provider_identifier,
                urlencoding::encode(&integration.provider_name),
            )))
        }
        Err(e) => {
            tracing::error!("OAuth callback failed: {e}");
            Ok(Redirect::to(&format!(
                "{}/?error={}",
                app_url,
                urlencoding::encode(&e.to_string()),
            )))
        }
    }
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

// ── Provider Status ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub identifier: String,
    pub name: String,
    pub configured: bool,
    pub oauth: bool,
    pub has_credentials: bool,
    pub editor_type: String,
    pub redirect_uri: String,
}

/// GET /api/providers — list all providers with config status
pub async fn list_providers(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<Vec<ProviderStatus>>, AppError> {
    let all = state.providers.all();
    let mut statuses = Vec::new();
    for provider in all {
        let id = provider.identifier();
        let has_creds = state.config.provider_credentials(id).is_some();
        statuses.push(ProviderStatus {
            identifier: id.to_string(),
            name: provider.name().to_string(),
            configured: has_creds,
            oauth: provider.uses_oauth(),
            has_credentials: has_creds,
            editor_type: format!("{:?}", provider.editor_type()),
            redirect_uri: if provider.uses_oauth() {
                format!("{}/api/auth/callback", state.config.app_url)
            } else {
                "N/A (non-OAuth)".into()
            },
        });
    }
    statuses.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    Ok(Json(statuses))
}
// ── Timeslots & Disable ─────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateTimeslotsRequest {
    pub timeslots: Vec<TimeslotEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeslotEntry {
    pub time: i32, // minutes from midnight
}

/// PUT /api/integrations/{id}/timeslots — update posting time slots
pub async fn update_timeslots(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTimeslotsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify integration belongs to user
    let _integration = queries::get_integration(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    // Validate: max 3 slots
    if body.timeslots.len() > 3 {
        return Err(AppError::BadRequest(
            "Maximum 3 time slots allowed".into(),
        ));
    }

    // Validate each time is 0-1439
    for slot in &body.timeslots {
        if slot.time < 0 || slot.time >= 1440 {
            return Err(AppError::BadRequest(
                "Invalid time: must be 0-1439 minutes from midnight".into(),
            ));
        }
    }

    let timeslots_json = serde_json::to_value(&body.timeslots)
        .map_err(|_| AppError::Internal("Failed to serialize timeslots".into()))?;

    sqlx::query(
        "UPDATE integrations SET posting_times = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(&timeslots_json)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "posting_times": timeslots_json
    })))
}

#[derive(Debug, Deserialize)]
pub struct ToggleDisableRequest {
    pub disabled: bool,
}

/// PUT /api/integrations/{id}/disable — toggle integration disabled state
pub async fn toggle_disable(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ToggleDisableRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _integration = queries::get_integration(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    sqlx::query("UPDATE integrations SET disabled = $1, updated_at = NOW() WHERE id = $2")
        .bind(body.disabled)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"success": true, "disabled": body.disabled})))
}

// ── Multi-Account Pages API ──────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AvailablePagesResponse {
    pub pages: Vec<PageInfo>,
    pub parent_integration_id: String,
    pub provider: String,
}

/// GET /api/integrations/{id}/available-pages — list sub-accounts for a provider
///
/// Uses provider.pages() to discover connectable pages/channels.
/// For multi-step providers (isBetweenSteps), access_token is the user-level token.
pub async fn available_pages(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AvailablePagesResponse>, AppError> {
    let integration = queries::get_integration(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider_obj = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| AppError::BadRequest("Provider not found in registry".into()))?;

    // Decrypt the stored token if encryption is enabled
    let resolve_token = |token: &str| -> String {
        state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(token, key).ok())
            .unwrap_or_else(|| token.to_string())
    };

    // Facebook/Instagram store the user-level token in refresh_token for page discovery.
    // Other multi-step providers (LinkedIn Page) use access_token directly.
    let raw_token = if integration.provider_identifier == "facebook"
        || integration.provider_identifier == "instagram"
    {
        integration
            .refresh_token
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or(&integration.access_token)
    } else {
        &integration.access_token
    };
    let token = resolve_token(raw_token);

    let pages = provider_obj
        .pages(&token)
        .await
        .map_err(|e| AppError::Provider(format!("Failed to list pages: {e}")))?;

    Ok(Json(AvailablePagesResponse {
        pages,
        parent_integration_id: integration.id.to_string(),
        provider: integration.provider_identifier,
    }))
}

#[derive(Debug, Serialize)]
pub struct ConnectPageResponse {
    pub integration: IntegrationPublic,
    pub parent_id: String,
}

/// POST /api/integrations/{parent_id}/connect-page/{page_id} — connect a sub-account
///
/// Creates a new integration linked to the parent via root_internal_id.
/// The page token is obtained from provider.pages().
pub async fn connect_page(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((parent_id, page_id)): Path<(Uuid, String)>,
) -> Result<Json<ConnectPageResponse>, AppError> {
    let parent = queries::get_integration(&state.db, parent_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider_obj = state
        .providers
        .get(&parent.provider_identifier)
        .ok_or_else(|| AppError::BadRequest("Provider not found in registry".into()))?;

    let resolve_token = |token: &str| -> String {
        state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(token, key).ok())
            .unwrap_or_else(|| token.to_string())
    };

    // Same token discovery logic as available_pages
    let raw_token = if parent.provider_identifier == "facebook"
        || parent.provider_identifier == "instagram"
    {
        parent
            .refresh_token
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or(&parent.access_token)
    } else {
        &parent.access_token
    };
    let token = resolve_token(raw_token);

    // Fetch all pages and find the matching one
    let pages = provider_obj
        .pages(&token)
        .await
        .map_err(|e| AppError::Provider(format!("Failed to list pages: {e}")))?;

    let page = pages
        .into_iter()
        .find(|p| p.id == page_id)
        .ok_or_else(|| AppError::BadRequest("Page not found or not accessible with this token".into()))?;

    let page_token = page.access_token.unwrap_or_default();

    let integration = queries::create_integration(
        &state.db,
        auth.user_id,
        &parent.provider_identifier,
        provider_obj.name(),
        &page.id,
        &page_token,
        parent.refresh_token.as_deref(),
        None,
        Some(&page.name),
        page.picture.as_deref(),
        None,
        Some(&parent.internal_id),
    )
    .await?;

    state.broadcast.send(
        "integration_connected",
        &serde_json::json!({
            "id": integration.id.to_string(),
            "provider": parent.provider_identifier,
            "parent_id": parent_id.to_string(),
        }),
    );

    tracing::info!(
        "Sub-account connected: {} page '{}' ({}) under parent {}",
        parent.provider_identifier,
        page.name,
        page.id,
        parent.internal_id,
    );

    Ok(Json(ConnectPageResponse {
        integration: integration.into(),
        parent_id: parent.id.to_string(),
    }))
}
