// ─── MCP Integration Tools ────────────────────────────────────
// Tool handlers for social media channel management.
// Thin wrappers over the shared IntegrationService.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::services::integrations::IntegrationService;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListIntegrationsInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListProvidersInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInfo {
    pub identifier: String,
    pub name: String,
    pub configured: bool,
    pub oauth: bool,
    pub has_credentials: bool,
    pub editor_type: String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListProvidersOutput {
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IntegrationInfo {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub profile_name: Option<String>,
    pub disabled: bool,
    pub refresh_needed: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListIntegrationsOutput {
    pub integrations: Vec<IntegrationInfo>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConnectInput {
    pub provider: String,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConnectOutput {
    pub auth_url: String,
    pub state: String,
    pub instructions: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DisconnectInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConnectCompleteInput {
    pub code: String,
    pub state: String,
}

// ── Tool Implementations ────────────────────────────────────

pub async fn list_providers(
    state: &AppState,
    _input: &ListProvidersInput,
) -> Result<Json<ListProvidersOutput>, String> {
    // Verify user exists (auth gate)
    let _user_id = super::tools_posts::resolve_first_user(state).await?;

    let all = state.providers.all();
    let mut providers: Vec<ProviderInfo> = all
        .into_iter()
        .map(|p| {
            let id = p.identifier();
            let has_creds = state.config.provider_credentials(id).is_some();
            ProviderInfo {
                identifier: id.to_string(),
                name: p.name().to_string(),
                configured: has_creds,
                oauth: p.uses_oauth(),
                has_credentials: has_creds,
                editor_type: format!("{:?}", p.editor_type()),
                redirect_uri: if p.uses_oauth() {
                    format!("{}/api/auth/callback", state.config.app_url)
                } else {
                    "N/A (non-OAuth)".into()
                },
            }
        })
        .collect();
    providers.sort_by(|a, b| a.identifier.cmp(&b.identifier));

    Ok(Json(ListProvidersOutput { providers }))
}

pub async fn list_integrations(
    state: &AppState,
    _input: &ListIntegrationsInput,
) -> Result<Json<ListIntegrationsOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let integrations = IntegrationService::list(&state.db, user_id).await?;

    let list: Vec<IntegrationInfo> = integrations
        .into_iter()
        .map(|i| IntegrationInfo {
            id: i.id.to_string(),
            provider: i.provider_identifier,
            name: i.provider_name,
            profile_name: i.profile_name,
            disabled: i.disabled,
            refresh_needed: i.refresh_needed,
        })
        .collect();

    Ok(Json(ListIntegrationsOutput { integrations: list }))
}

pub async fn connect_integration(
    state: &AppState,
    input: &ConnectInput,
) -> Result<Json<ConnectOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let provider_obj = state
        .providers
        .get(&input.provider)
        .ok_or_else(|| format!("Unknown provider: {}", input.provider))?;

    // Handle non-OAuth providers (e.g., Bluesky with app passwords)
    if !provider_obj.uses_oauth() {
        // One-time-token providers (Telegram): return instructions
        if provider_obj.one_time_token() {
            let code = uuid::Uuid::new_v4().to_string();
            return Ok(Json(ConnectOutput {
                auth_url: format!(
                    "Open Telegram, start a chat with this bot, and send: /connect {}",
                    code
                ),
                state: "one-time-token".into(),
                instructions: format!(
                    "Send /connect {} to your Telegram bot to link this channel.",
                    code,
                ),
            }));
        }

        let redirect_uri = input
            .redirect_uri
            .clone()
            .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

        let token = provider_obj
            .exchange_code("", "", &redirect_uri)
            .await
            .map_err(|e| format!("Failed to connect {}: {e}", input.provider))?;

        crate::db::queries::create_integration(
            &state.db,
            user_id,
            &input.provider,
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
        .await
        .map_err(|e| e.to_string())?;

        return Ok(Json(ConnectOutput {
            auth_url: format!("Connected to {} as {}. No browser OAuth needed.", input.provider, token.name),
            state: "auto".into(),
            instructions: format!(
                "Connected to {} as {}. Store is complete.",
                input.provider, token.name
            ),
        }));
    }

    let result = IntegrationService::initiate_connect(
        &state.db,
        &state.providers,
        user_id,
        &input.provider,
        &state.config.app_url,
    ).await?;

    Ok(Json(ConnectOutput {
        auth_url: result.auth_url,
        state: result.state,
        instructions: format!(
            "Open the auth_url in a browser, authorize the app, then \
             the callback URL will complete the connection. \
             For MCP flow, after authorization the callback at {} \
             will process the result.",
            state.config.app_url,
        ),
    }))
}

pub async fn complete_connect_integration(
    state: &AppState,
    input: &ConnectCompleteInput,
) -> Result<Json<super::SuccessOutput>, String> {
    let _integration = IntegrationService::complete_connect(
        &state.db,
        &state.providers,
        &state.broadcast,
        &input.state,
        &input.code,
        state.token_key.as_ref(),
    )
    .await?;

    Ok(Json(super::SuccessOutput {
        success: true,
        message: "Integration connected successfully".into(),
    }))
}

pub async fn disconnect_integration(
    state: &AppState,
    input: &DisconnectInput,
) -> Result<Json<super::SuccessOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let integration_id =
        uuid::Uuid::parse_str(&input.id).map_err(|_| "Invalid integration ID".to_string())?;

    let deleted = IntegrationService::disconnect(
        &state.db,
        &state.broadcast,
        user_id,
        integration_id,
    ).await?;

    if !deleted {
        return Err("Integration not found".into());
    }

    Ok(Json(super::SuccessOutput {
        success: true,
        message: "Integration disconnected".into(),
    }))
}
