// ─── MCP Integration Tools ────────────────────────────────────
// Tool handlers for social media channel management.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::db::queries;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListIntegrationsInput {
    pub token: Option<String>,
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
    pub token: Option<String>,
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
    pub token: Option<String>,
}

// ── Tool Implementations ────────────────────────────────────

pub async fn list_integrations(
    state: &AppState,
    _input: &ListIntegrationsInput,
) -> Result<Json<ListIntegrationsOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let integrations = queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| e.to_string())?;

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
        let redirect_uri = input
            .redirect_uri
            .clone()
            .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

        let token = provider_obj
            .exchange_code("", "", &redirect_uri)
            .await
            .map_err(|e| format!("Failed to connect {}: {e}", input.provider))?;

        queries::create_integration(
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

    let code_verifier = crate::social::common::generate_code_verifier();
    let oauth_state = crate::social::common::generate_state();
    let redirect_uri = input
        .redirect_uri
        .clone()
        .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

    let auth_url = provider_obj
        .generate_auth_url(&oauth_state, &code_verifier, &redirect_uri)
        .await
        .map_err(|e| format!("Failed to generate auth URL: {e}"))?;

    // Store OAuth state
    queries::save_oauth_state(
        &state.db,
        &oauth_state,
        &input.provider,
        &code_verifier,
        Some(&format!("{}:{}", user_id, redirect_uri)),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(Json(ConnectOutput {
        auth_url: auth_url.url,
        state: oauth_state,
        instructions: format!(
            "Open the auth_url in a browser, authorize the app, then \
             the callback URL will complete the connection. \
             For MCP flow, after authorization the callback at {} \
             will process the result.",
            redirect_uri
        ),
    }))
}

pub async fn disconnect_integration(
    state: &AppState,
    input: &DisconnectInput,
) -> Result<Json<super::SuccessOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let integration_id =
        uuid::Uuid::parse_str(&input.id).map_err(|_| "Invalid integration ID".to_string())?;

    let deleted = queries::delete_integration(&state.db, integration_id, user_id)
        .await
        .map_err(|e| e.to_string())?;

    if !deleted {
        return Err("Integration not found".into());
    }

    state
        .broadcast
        .send("integration_disconnected", &serde_json::json!({"id": input.id}));

    Ok(Json(super::SuccessOutput {
        success: true,
        message: "Integration disconnected".into(),
    }))
}
