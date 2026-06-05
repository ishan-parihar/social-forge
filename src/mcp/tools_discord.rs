// ─── MCP Discord Tools ───────────────────────────────────────────
// Discord Bot API tools for reading channels, messages, guilds, and thread members.
// Uses Bot token auth internally (from config.discord_bot_token), with OAuth user
// token for channel/guild association lookup.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::discord::DiscordProvider;

// ── Input Types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiGetChannelInput {
    pub channel_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiGetMessagesInput {
    pub channel_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiGetGuildInput {
    pub channel_id: String,
    pub guild_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiGetThreadMembersInput {
    pub channel_id: String,
    pub thread_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

/// Find a Discord integration by channel_id (stored as internal_id) and return its access token.
/// The bot_token is not returned — it is read from config by DiscordProvider internally.
async fn find_di_token(state: &AppState, user_id: Uuid, channel_id: &str) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let discord = integrations
        .iter()
        .find(|i| i.provider_identifier == "discord" && i.internal_id == channel_id)
        .ok_or_else(|| {
            format!(
                "Discord channel '{channel_id}' not connected. Connect Discord first via integrations_connect."
            )
        })?;

    let __tok = discord.access_token.clone();
    let __tok = state.token_key.as_ref()
        .and_then(|k| crate::crypto::decrypt_string(&__tok, k).ok())
        .unwrap_or(__tok);
    Ok(__tok)
}

/// Create a DiscordProvider from the app config (bot_token sourced from config.discord_bot_token).
fn create_di_provider(state: &AppState) -> DiscordProvider {
    DiscordProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_di_get_channel(
    state: &AppState,
    input: &DiGetChannelInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_di_token(state, user_id, &input.channel_id).await?;
    let provider = create_di_provider(state);
    let result = provider
        .get_channel(&token, &input.channel_id)
        .await
        .map_err(|e| format!("Discord get channel failed: {e}"))?;
    Ok(Json(json!(result)))
}

pub async fn handle_di_get_messages(
    state: &AppState,
    input: &DiGetMessagesInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_di_token(state, user_id, &input.channel_id).await?;
    let provider = create_di_provider(state);
    let result = provider
        .get_channel_messages(&token, &input.channel_id, input.limit.unwrap_or(50))
        .await
        .map_err(|e| format!("Discord get messages failed: {e}"))?;
    Ok(Json(json!(result)))
}

pub async fn handle_di_get_guild(
    state: &AppState,
    input: &DiGetGuildInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_di_token(state, user_id, &input.channel_id).await?;
    let provider = create_di_provider(state);
    let result = provider
        .get_guild(&token, &input.guild_id)
        .await
        .map_err(|e| format!("Discord get guild failed: {e}"))?;
    Ok(Json(json!(result)))
}

pub async fn handle_di_get_thread_members(
    state: &AppState,
    input: &DiGetThreadMembersInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_di_token(state, user_id, &input.channel_id).await?;
    let provider = create_di_provider(state);
    let result = provider
        .get_thread_members(&token, &input.thread_id)
        .await
        .map_err(|e| format!("Discord get thread members failed: {e}"))?;
    Ok(Json(json!(result)))
}

// ── New Bot-API Tools (no OAuth token needed) ──────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiSendMessageInput {
    pub channel_id: String,
    pub content: String,
}

pub async fn handle_di_send_message(
    state: &AppState,
    input: &DiSendMessageInput,
) -> Result<Json<Value>, String> {
    let provider = create_di_provider(state);
    provider
        .send_message(&input.channel_id, &input.content)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiDeleteMessageInput {
    pub channel_id: String,
    pub message_id: String,
}

pub async fn handle_di_delete_message(
    state: &AppState,
    input: &DiDeleteMessageInput,
) -> Result<Json<Value>, String> {
    let provider = create_di_provider(state);
    provider
        .delete_message(&input.channel_id, &input.message_id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiAddReactionInput {
    pub channel_id: String,
    pub message_id: String,
    pub emoji: String,
}

pub async fn handle_di_add_reaction(
    state: &AppState,
    input: &DiAddReactionInput,
) -> Result<Json<Value>, String> {
    let provider = create_di_provider(state);
    provider
        .add_reaction(&input.channel_id, &input.message_id, &input.emoji)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiGetGuildChannelsInput {
    pub guild_id: String,
}

pub async fn handle_di_get_guild_channels(
    state: &AppState,
    input: &DiGetGuildChannelsInput,
) -> Result<Json<Value>, String> {
    let provider = create_di_provider(state);
    provider
        .get_guild_channels(&input.guild_id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiGetServerInfoInput {
    pub guild_id: String,
}

pub async fn handle_di_get_server_info(
    state: &AppState,
    input: &DiGetServerInfoInput,
) -> Result<Json<Value>, String> {
    let provider = create_di_provider(state);
    provider
        .get_server_info(&input.guild_id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiCreateForumPostInput {
    pub channel_id: String,
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub applied_tags: Vec<String>,
}

pub async fn handle_di_create_forum_post(
    state: &AppState,
    input: &DiCreateForumPostInput,
) -> Result<Json<Value>, String> {
    let provider = create_di_provider(state);
    provider
        .create_forum_post(&input.channel_id, &input.name, &input.content, &input.applied_tags)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}
