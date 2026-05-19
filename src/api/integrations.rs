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
            let code = uuid::Uuid::new_v4().to_string();
            let instructions = match provider.as_str() {
                "telegram-bot" => {
                    // Get bot username from configured token
                    let mut bot_username = String::from("your bot");
                    if let Some((_, tokens_str)) = state.config.provider_credentials("telegram-bot") {
                        let token = tokens_str.split(',').next().unwrap_or("").trim().to_string();
                        if !token.is_empty() {
                            if let Ok(resp) = reqwest::Client::new()
                                .get(format!("https://api.telegram.org/bot{token}/getMe"))
                                .send().await
                            {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    if let Some(u) = json["result"]["username"].as_str() {
                                        bot_username = format!("@{u}");
                                    }
                                }
                            }
                        }
                    }
                    format!("{bot_username}\n/connect {code}")
                },
                "telegram-user" => "Use MCP tools (tu_request_code → tu_sign_in) to authenticate your Telegram account, then click Verify.".into(),
                "whatsapp" => "Use MCP tools (wa_pair_code) to link your WhatsApp device, then click Verify.".into(),
                _ => format!("Send this code to the provider: /connect {code}"),
            };
            return Ok(Json(ConnectResponse {
                url: instructions,
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
        None, // auth_method
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

    tracing::info!("OAuth connect {provider}: redirect_uri={redirect_uri}");

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
                        "{}/auth/callback?pending={}&integration_id={}&token={}",
                        app_url,
                        integration.provider_identifier,
                        integration.id,
                        token,
                    )));
                }
            }

            Ok(Redirect::to(&format!(
                "{}/auth/callback?connected={}&name={}",
                app_url,
                integration.provider_identifier,
                urlencoding::encode(&integration.provider_name),
            )))
        }
        Err(e) => {
            tracing::error!("OAuth callback failed: {e}");
            Ok(Redirect::to(&format!(
                "{}/auth/callback?error={}",
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
                format!("{}/api/auth/callback", state.config.frontend_url)
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

    // Validate no duplicate times
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for slot in &body.timeslots {
        if !seen.insert(&slot.time) {
            return Err(AppError::BadRequest(
                format!("Duplicate time slot: {} minutes is already set", slot.time)
            ));
        }
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
        "UPDATE integrations SET posting_times = $1, updated_at = NOW() WHERE id = $2 AND user_id = $3",
    )
    .bind(&timeslots_json)
    .bind(id)
    .bind(auth.user_id)
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

    sqlx::query("UPDATE integrations SET disabled = $1, updated_at = NOW() WHERE id = $2 AND user_id = $3")
        .bind(body.disabled)
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"success": true, "disabled": body.disabled})))
}

/// POST /api/integrations/{id}/refresh — re-fetch profile info from provider
pub async fn refresh(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let integration = queries::get_integration(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider_obj = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| AppError::BadRequest("Provider not found in registry".into()))?;

    let resolve_token = |token: &str| -> String {
        state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(token, key).ok())
            .unwrap_or_else(|| token.to_string())
    };

    let token = resolve_token(&integration.access_token);

    let info = provider_obj
        .reconnect(&token, &integration.internal_id, &integration.internal_id)
        .await
        .map_err(|e| AppError::Provider(format!("Failed to refresh: {e}")))?;

    sqlx::query(
        "UPDATE integrations SET profile_name = $1, profile_picture = COALESCE($2, profile_picture), updated_at = NOW() WHERE id = $3 AND user_id = $4",
    )
    .bind(&info.name)
    .bind(info.picture.clone())
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "profile_name": info.name,
        "profile_picture": info.picture,
    })))
}

// ── X Cookie Connect ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectXCookieRequest {
    pub auth_token: String,
    pub ct0: String,
}

/// POST /api/integrations/connect/:provider/verify — verify one-time-token (Telegram Bot)
pub async fn verify_one_time_token(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(provider): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ConnectResponse>, AppError> {
    let code = body["code"].as_str().unwrap_or("").to_string();

    let provider_obj = state
        .providers
        .get(&provider)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider: {provider}")))?;

    let redirect_uri = format!("{}/api/auth/callback", state.config.app_url);
    let token = provider_obj
        .exchange_code(&code, "", &redirect_uri)
        .await
        .map_err(|e| AppError::Provider(format!("Verification failed: {e}")))?;

    queries::create_integration(
        &state.db,
        auth.user_id,
        &provider,
        &provider_obj.name(),
        &token.provider_user_id,
        &token.access_token,
        token.refresh_token.as_deref(),
        token.expires_in.map(|exp| chrono::Utc::now() + chrono::Duration::seconds(exp as i64)),
        Some(&token.name),
        token.picture.as_deref(),
        token.username.is_empty().then_some(None).unwrap_or(Some(&format!("@{}", token.username))),
        None,
        None,
    )
    .await?;

    state.broadcast.send("integration_connected", &serde_json::json!({ "provider": provider, "name": token.name }));

    Ok(Json(ConnectResponse {
        url: format!("Connected to {} as {}", provider_obj.name(), token.name),
        state: "connected".into(),
    }))
}

/// POST /api/integrations/connect/telegram-bot/token — connect with a custom bot token
pub async fn connect_telegram_bot_token(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = body["token"].as_str().unwrap_or("").trim().to_string();
    if token.is_empty() {
        return Err(AppError::BadRequest("Bot token required".into()));
    }

    // Validate token via getMe
    let http = reqwest::Client::new();
    let resp = http.get(format!("https://api.telegram.org/bot{token}/getMe"))
        .send().await
        .map_err(|e| AppError::Provider(format!("Failed to reach Telegram: {e}")))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Provider(format!("Invalid response: {e}")))?;

    if !json["ok"].as_bool().unwrap_or(false) {
        return Err(AppError::Provider("Invalid bot token".into()));
    }

    let bot = &json["result"];
    let bot_id = bot["id"].as_i64().unwrap_or(0).to_string();
    let bot_username = bot["username"].as_str().unwrap_or("");
    let display_name = if bot_username.is_empty() { "Telegram Bot".to_string() } else { format!("@{bot_username}") };
    let profile_url = if bot_username.is_empty() { None } else { Some(format!("@{bot_username}")) };

    // Store as JSON with bot_token (no chat_id yet — user connects a chat via /connect flow)
    let access_token = serde_json::json!({ "bot_token": token }).to_string();

    let integration = queries::create_integration(
        &state.db, auth.user_id,
        "telegram-bot", "Telegram Bot",
        &bot_id, &access_token,
        None, None,
        Some(&display_name),
        None,
        profile_url.as_deref(),
        None, None,
    ).await?;

    let public: IntegrationPublic = integration.into();
    Ok(Json(serde_json::json!({ "integration": public })))
}

/// POST /api/integrations/connect/whatsapp/pair — request pair code
pub async fn whatsapp_pair(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let phone = body["phone_number"].as_str().unwrap_or("").trim().to_string();
    if phone.is_empty() || !phone.starts_with('+') {
        return Err(AppError::BadRequest("phone_number required in international format (e.g. +1234567890)".into()));
    }

    let wa = state.wa_client.as_ref()
        .ok_or_else(|| AppError::Provider("WhatsApp client not configured".into()))?;

    // Force fresh connection for reliable pair code generation
    {
        let mut locked = wa.lock().await;
        locked.reconnect().await.map_err(|e| AppError::Provider(format!("WhatsApp connect failed: {e}")))?;
    }

    let code = crate::wa::auth::pair_with_code(wa, crate::wa::auth::PairOptions {
        phone_number: phone,
        show_push_notification: true,
    }).await.map_err(|e| AppError::Provider(format!("Pair code request failed: {e}")))?;

    Ok(Json(serde_json::json!({ "pair_code": code, "expires_in": 180 })))
}

/// GET /api/integrations/connect/whatsapp/status — poll auth status
pub async fn whatsapp_status(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let wa = state.wa_client.as_ref()
        .ok_or_else(|| AppError::Provider("WhatsApp client not configured".into()))?;

    let locked = wa.lock().await;
    let authenticated = locked.is_authenticated();
    let jid = if authenticated {
        locked.inner().get_pn().await.map(|j| j.to_string())
    } else {
        None
    };
    Ok(Json(serde_json::json!({ "authenticated": authenticated, "jid": jid })))
}

/// POST /api/integrations/connect/telegram-user/request-code
pub async fn telegram_user_request_code(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let phone = body["phone"].as_str().unwrap_or("").trim().to_string();
    if phone.is_empty() {
        return Err(AppError::BadRequest("phone number required".into()));
    }

    let mgr = state.telegram_client_manager.as_ref()
        .ok_or_else(|| AppError::Provider("Telegram user client not configured. Set TELEGRAM_API_ID and TELEGRAM_API_HASH.".into()))?;

    mgr.request_login_code(&phone).await
        .map_err(|e| AppError::Provider(format!("Failed to send code: {e}")))?;

    Ok(Json(serde_json::json!({ "status": "code_sent" })))
}

/// POST /api/integrations/connect/telegram-user/sign-in
pub async fn telegram_user_sign_in(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let code = body["code"].as_str().unwrap_or("").trim().to_string();
    if code.is_empty() {
        return Err(AppError::BadRequest("verification code required".into()));
    }

    let mgr = state.telegram_client_manager.as_ref()
        .ok_or_else(|| AppError::Provider("Telegram user client not configured".into()))?;

    mgr.sign_in("", &code).await
        .map_err(|e| AppError::Provider(format!("Sign in failed: {e}")))?;

    // Fetch user info and create integration
    let info = mgr.user_info().await
        .map_err(|e| AppError::Provider(format!("Failed to get user info: {e}")))?;

    let user_id = info["id"].as_i64().unwrap_or(0).to_string();
    let name = info["name"].as_str().unwrap_or("Telegram User").to_string();
    let username = info["username"].as_str().unwrap_or("").to_string();
    let profile_url = if username.is_empty() { None } else { Some(format!("@{username}")) };

    let integration = queries::create_integration(
        &state.db, auth.user_id,
        "telegram-user", "Telegram User",
        &user_id, &user_id,
        None, None,
        Some(&name),
        None,
        profile_url.as_deref(),
        None, None,
    ).await?;

    let public: IntegrationPublic = integration.into();
    Ok(Json(serde_json::json!({ "integration": public })))
}

/// POST /api/integrations/connect/x-cookie — connect X via browser cookies
pub async fn connect_x_cookie(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<ConnectXCookieRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.auth_token.trim().is_empty() || body.ct0.trim().is_empty() {
        return Err(AppError::BadRequest("auth_token and ct0 are required".into()));
    }

    let cookie_str = format!("auth_token={}; ct0={};", body.auth_token.trim(), body.ct0.trim());
    let ct0 = body.ct0.trim();
    let bearer = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

    let http = reqwest::Client::new();

    // Step 1: Get current user from multi/list
    let resp = http
        .get("https://x.com/i/api/1.1/account/multi/list.json")
        .header("x-csrf-token", ct0)
        .header("Cookie", &cookie_str)
        .header("Authorization", bearer)
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("X cookie auth failed: {e}")))?;

    let list: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Provider(format!("X response parse error: {e}")))?;

    let users = list["users"].as_array()
        .ok_or_else(|| AppError::Provider("X cookie auth: no users in response".into()))?;
    let user = users.first()
        .ok_or_else(|| AppError::Provider("X cookie auth: empty users list".into()))?;

    let screen_name = user["screen_name"].as_str().unwrap_or("");
    let user_id_str = user["user_id"].as_str().unwrap_or("");

    if screen_name.is_empty() || user_id_str.is_empty() {
        return Err(AppError::Provider("X cookie auth: could not determine user".into()));
    }

    // Step 2: Get full profile via GraphQL UserByScreenName
    let vars = serde_json::json!({"screen_name": screen_name, "withSafetyModeUserFields": true});
    let features = serde_json::json!({"hidden_profile_subscriptions_enabled":true,"responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,"responsive_web_graphql_skip_user_profile_image_extensions_enabled":false,"responsive_web_graphql_timeline_navigation_enabled":true});
    let qid = "1VOOyvKkiI3FMmkeDNxM9A";
    let gql_url = format!(
        "https://x.com/i/api/graphql/{qid}/UserByScreenName?variables={}&features={}",
        urlencoding::encode(&vars.to_string()),
        urlencoding::encode(&features.to_string()),
    );

    let resp = http
        .get(&gql_url)
        .header("x-csrf-token", ct0)
        .header("Cookie", &cookie_str)
        .header("Authorization", bearer)
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("X GraphQL error: {e}")))?;

    let profile: serde_json::Value = resp.json().await.unwrap_or_default();
    let legacy = profile.pointer("/data/user/result/legacy");
    let name = legacy.and_then(|l| l["name"].as_str()).unwrap_or(screen_name);
    let picture = legacy.and_then(|l| l["profile_image_url_https"].as_str())
        .map(|u| u.replace("_normal", "_400x400"));

    // Store as JSON blob
    let token_json = serde_json::json!({
        "auth_token": body.auth_token.trim(),
        "ct0": body.ct0.trim(),
        "cookie_string": cookie_str,
    });
    let access_token = token_json.to_string();

    let integration = queries::create_integration(
        &state.db,
        auth.user_id,
        "x",
        "X (Twitter)",
        user_id_str,
        &access_token,
        None,
        None,
        Some(name),
        picture.as_deref(),
        Some(&format!("@{screen_name}")),
        None,
        Some("cookie"),
    )
    .await?;

    let public: IntegrationPublic = integration.into();
    Ok(Json(serde_json::json!({ "integration": public })))
}

// ── GitHub PAT Connect ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectGithubPatRequest {
    pub pat: String,
    pub label: Option<String>,
}

/// POST /api/integrations/connect/github-pat — connect GitHub via Personal Access Token
pub async fn connect_github_pat(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<ConnectGithubPatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.pat.trim().is_empty() {
        return Err(AppError::BadRequest("pat is required".into()));
    }

    let http = reqwest::Client::new();
    let resp = http
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", body.pat.trim()))
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "social-forge:v0.1.0")
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("GitHub API error: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::BadRequest("Invalid GitHub PAT or insufficient permissions".into()));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Provider(format!("GitHub response parse error: {e}")))?;

    let gh_id = json["id"].as_u64().map(|n| n.to_string()).unwrap_or_default();
    let login = json["login"].as_str().unwrap_or("");
    let name = json["name"].as_str().or(Some(login)).unwrap_or("");
    let avatar = json["avatar_url"].as_str();

    if gh_id.is_empty() {
        return Err(AppError::Provider("Failed to fetch GitHub user info".into()));
    }

    let label = body.label.as_deref().unwrap_or(name);

    let integration = queries::create_integration(
        &state.db,
        auth.user_id,
        "github",
        "GitHub",
        &gh_id,
        body.pat.trim(),
        None,
        None,
        Some(label),
        avatar,
        Some(&format!("@{login}")),
        None,
        Some("pat"),
    )
    .await?;

    let public: IntegrationPublic = integration.into();
    Ok(Json(serde_json::json!({ "integration": public })))
}

// ── API Key Connect ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectApiKeyRequest {
    pub provider: String,
    pub api_key: String,
    pub instance_url: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectApiKeyResponse {
    pub integration: IntegrationPublic,
}

/// POST /api/integrations/connect/api-key — connect a provider using API key
///
/// For providers like Lemmy that use per-user API keys + instance URLs.
/// Validates the API key by calling the provider's pages() method,
/// then stores the credentials as JSON in the integration record.
pub async fn connect_api_key(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<ConnectApiKeyRequest>,
) -> Result<Json<ConnectApiKeyResponse>, AppError> {
    let provider_obj = state
        .providers
        .get(&body.provider)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider: {}", body.provider)))?;

    if provider_obj.uses_oauth() {
        return Err(AppError::BadRequest(
            format!("Provider '{}' uses OAuth, not API key auth. Use the standard connect flow.", body.provider)
        ));
    }

    if body.api_key.trim().is_empty() {
        return Err(AppError::BadRequest("api_key must not be empty".into()));
    }

    // Build the credential JSON (WordPress-style per-user credential storage)
    let instance_url = body.instance_url.unwrap_or_default();
    let label = body.label.unwrap_or_else(|| provider_obj.name().to_string());

    let creds_json = serde_json::json!({
        "api_key": body.api_key,
        "instance_url": instance_url,
    });
    let access_token = creds_json.to_string();

    // Validate by calling provider's pages() method
    let pages = provider_obj
        .pages(&access_token)
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to validate API key: {e}")))?;

    let page = pages.into_iter().next().unwrap_or(PageInfo {
        id: provider_obj.identifier().to_string(),
        name: label.clone(),
        access_token: Some(access_token.clone()),
        picture: None,
        username: None,
    });

    let integration = queries::create_integration(
        &state.db,
        auth.user_id,
        &body.provider,
        provider_obj.name(),
        &page.id,
        &access_token,
        None,
        None,
        Some(&label),
        page.picture.as_deref(),
        None,
        None,
        None, // auth_method
    )
    .await?;

    state.broadcast.send(
        "integration_connected",
        &serde_json::json!({
            "provider": body.provider,
            "name": label,
        }),
    );

    tracing::info!(
        "API key integration connected: {} ({}) for user {}",
        body.provider,
        label,
        auth.user_id,
    );

    Ok(Json(ConnectApiKeyResponse {
        integration: integration.into(),
    }))
}

// ── Web3 Connect ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectWeb3Request {
    pub provider: String,
    pub address: String,
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectWeb3Response {
    pub integration: IntegrationPublic,
}

/// POST /api/integrations/connect/web3 — connect a Web3 provider
///
/// For providers like Farcaster and Nostr that use wallet/npub addresses.
/// Accepts { provider, address, label }, validates via exchange_code,
/// then stores the address as the access_token.
pub async fn connect_web3(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<ConnectWeb3Request>,
) -> Result<Json<ConnectWeb3Response>, AppError> {
    let provider_obj = state
        .providers
        .get(&body.provider)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider: {}", body.provider)))?;

    if provider_obj.uses_oauth() {
        return Err(AppError::BadRequest(
            format!("Provider '{}' uses OAuth, not Web3 auth. Use the standard connect flow.", body.provider)
        ));
    }

    // Verify provider is Web3-capable (farcaster or nostr only)
    if body.provider != "farcaster" && body.provider != "nostr" {
        return Err(AppError::BadRequest(
            format!("Web3 auth is not supported for provider '{}'. Only 'farcaster' and 'nostr' support this auth method.", body.provider)
        ));
    }

    // Check credentials first
    if state.config.provider_credentials(&body.provider).is_none() {
        return Err(AppError::BadRequest(
            format!("Provider '{}' is not configured. Set the corresponding environment variables first.", body.provider)
        ));
    }

    if body.address.trim().is_empty() {
        return Err(AppError::BadRequest("address must not be empty".into()));
    }

    let label = body.label.unwrap_or_else(|| provider_obj.name().to_string());

    // Build credential JSON and pass through exchange_code for validation
    let creds_json = serde_json::json!({
        "address": body.address,
    });
    let code = creds_json.to_string();

    let auth_result = provider_obj
        .exchange_code(&code, "", "")
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to validate Web3 credentials: {e}")))?;

    // Use the normalized address from exchange_code as the access_token
    let final_token = auth_result.access_token;

    let integration = queries::create_integration(
        &state.db,
        auth.user_id,
        &body.provider,
        provider_obj.name(),
        &auth_result.provider_user_id,
        &final_token,
        None,
        None,
        Some(&label),
        auth_result.picture.as_deref(),
        None,
        None,
        None, // auth_method
    )
    .await?;

    state.broadcast.send(
        "integration_connected",
        &serde_json::json!({
            "provider": body.provider,
            "name": label,
        }),
    );

    tracing::info!(
        "Web3 integration connected: {} ({}) for user {}",
        body.provider,
        label,
        auth.user_id,
    );

    Ok(Json(ConnectWeb3Response {
        integration: integration.into(),
    }))
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

    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM integrations WHERE user_id = $1 AND provider_identifier = $2 AND internal_id = $3",
    )
    .bind(auth.user_id)
    .bind(&parent.provider_identifier)
    .bind(&page.id)
    .fetch_one(&state.db)
    .await?;

    if existing > 0 {
        return Err(AppError::BadRequest(
            format!("Page '{}' is already connected", page.name)
        ));
    }

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
        None, // auth_method
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
