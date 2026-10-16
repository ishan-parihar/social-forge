// ─── MCP Discord Tools ───────────────────────────────────────────
// Discord Bot API tools for reading channels, messages, guilds, and thread members.
// Uses Bot token auth internally (from config.discord_bot_token), with OAuth user
// token for channel/guild association lookup.

use rmcp::{schemars::JsonSchema, Json};
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
async fn find_di_token(
    state: &AppState,
    user_id: Uuid,
    channel_id: &str,
) -> Result<String, String> {
    let discord = state
        .integration_by_internal_id(user_id, "discord", channel_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| {
            format!(
                "Discord channel '{channel_id}' not connected. Connect Discord first via integrations_connect."
            )
        })?;

    let tok = crate::crypto::maybe_decrypt_token(&discord.access_token, state.token_key.as_ref());
    Ok(tok)
}

/// Create a DiscordProvider from the app config (bot_token sourced from config.discord_bot_token).
fn create_di_provider(state: &AppState) -> DiscordProvider {
    DiscordProvider::new(&state.config)
}

// ── E7-read: history / search / tail ────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiHistoryInput {
    pub channel_id: String,
    /// Total messages to fetch (paginated 100/request). Default 200.
    pub limit: Option<u32>,
    /// Return messages OLDER than this message id (history walk).
    pub before: Option<String>,
    /// Return messages NEWER than this message id (incremental sync).
    pub after: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiSearchInput {
    pub guild_id: String,
    pub query: String,
    /// Restrict the search to one channel
    pub channel_id: Option<String>,
    /// Max hits to return (Discord returns groups of 25)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiTailInput {
    pub channel_id: String,
    /// Only messages newer than this message id (snapshot watch cursor)
    pub after: Option<String>,
    /// Default 25
    pub limit: Option<u32>,
}

/// Deep channel history, chronological (oldest→newest). Walks Discord's
/// 100-per-request cursor either way: `before` for backfill, `after` for
/// incremental sync.
pub async fn handle_di_history(
    state: &AppState,
    input: &DiHistoryInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    // The integration lookup is the connection gate; the Bot API reads its
    // credentials from config, so the token itself is unused here.
    let _token = find_di_token(state, user_id, &input.channel_id).await?;
    let provider = create_di_provider(state);
    let messages = provider
        .fetch_channel_history(
            &input.channel_id,
            input.limit.unwrap_or(200),
            input.before.as_deref(),
            input.after.as_deref(),
        )
        .await
        .map_err(|e| format!("Discord history failed: {e}"))?;
    Ok(Json(json!({
        "channel_id": input.channel_id,
        "count": messages.len(),
        "messages": messages,
    })))
}

/// Guild-wide message search (Discord built-in search).
pub async fn handle_di_search(
    state: &AppState,
    input: &DiSearchInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    // Search is guild-wide; a channel-scoped integration is not required,
    // so fall back to any connected Discord integration's token.
    // Connection gate only (Bot API creds come from config).
    let _token = match input.channel_id.as_deref() {
        Some(cid) if !cid.is_empty() => find_di_token(state, user_id, cid).await,
        _ => any_discord_token(state, user_id).await,
    }?;
    let provider = create_di_provider(state);
    let result = provider
        .search_guild_messages(
            &input.guild_id,
            &input.query,
            input.channel_id.as_deref(),
            input.limit.unwrap_or(25),
        )
        .await
        .map_err(|e| format!("Discord search failed: {e}"))?;
    Ok(Json(result))
}

/// Latest snapshot of a channel — the `tail -f` style watch cursor:
/// pass `after` = the newest message id from the previous call to get
/// only what arrived since.
pub async fn handle_di_tail(
    state: &AppState,
    input: &DiTailInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    // Connection gate only (Bot API creds come from config).
    let _token = find_di_token(state, user_id, &input.channel_id).await?;
    let provider = create_di_provider(state);
    let messages = provider
        .fetch_channel_history(
            &input.channel_id,
            input.limit.unwrap_or(25),
            None,
            input.after.as_deref(),
        )
        .await
        .map_err(|e| format!("Discord tail failed: {e}"))?;
    Ok(Json(json!({
        "channel_id": input.channel_id,
        "count": messages.len(),
        "messages": messages,
        "newest_id": messages.last().and_then(|m| m["id"].as_str()),
    })))
}

/// Any connected Discord integration's token — the guild-wide tools
/// (di_search) don't need a channel-scoped one.
async fn any_discord_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    integrations
        .iter()
        .find(|i| i.provider_identifier == "discord")
        .map(|i| crate::crypto::maybe_decrypt_token(&i.access_token, state.token_key.as_ref()))
        .ok_or_else(|| {
            "Discord not connected. Connect Discord first via integrations_connect.".to_string()
        })
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
        .create_forum_post(
            &input.channel_id,
            &input.name,
            &input.content,
            &input.applied_tags,
        )
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

// ── Webhook Tools (E7: zero-setup posting — no bot, no OAuth) ──

/// Only discord.com/discordapp.com webhook URLs — the tool takes a
/// caller-supplied URL, so a host guard keeps it from becoming a
/// post-anything SSRF gadget.
fn ensure_discord_webhook_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|_| "Invalid webhook URL".to_string())?;
    let host = parsed.host_str().unwrap_or_default();
    let ok = parsed.scheme() == "https"
        && (host == "discord.com" || host == "discordapp.com")
        && parsed.path().starts_with("/api/webhooks/");
    if !ok {
        return Err(
            "Not a Discord webhook URL (expected https://discord.com/api/webhooks/...)"
                .to_string(),
        );
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiWebhookPostInput {
    /// Discord webhook URL (https://discord.com/api/webhooks/...)
    pub webhook_url: String,
    pub content: String,
    /// Override the webhook's display name for this message (optional)
    pub username: Option<String>,
    /// Override the webhook's avatar for this message (optional)
    pub avatar_url: Option<String>,
    /// true = wait for the delivered message and return its id
    #[serde(default)]
    pub wait: Option<bool>,
}

pub async fn handle_di_webhook_post(
    state: &AppState,
    input: &DiWebhookPostInput,
) -> Result<Json<Value>, String> {
    ensure_discord_webhook_url(&input.webhook_url)?;
    let mut body = json!({ "content": input.content });
    if let Some(u) = &input.username {
        body["username"] = json!(u);
    }
    if let Some(a) = &input.avatar_url {
        body["avatar_url"] = json!(a);
    }
    let url = if input.wait.unwrap_or(false) {
        format!("{}?wait=true", input.webhook_url)
    } else {
        input.webhook_url.clone()
    };
    let resp = state
        .media_http_client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Webhook post failed: {e}"))?;
    let status = resp.status();
    let payload: Value = resp
        .json()
        .await
        .unwrap_or(Value::Null); // 204 No Content has no body
    if !status.is_success() {
        return Err(format!("Discord rejected the webhook ({status}): {payload}"));
    }
    Ok(Json(payload))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiWebhookInfoInput {
    /// Discord webhook URL to inspect (validates it and shows where it posts)
    pub webhook_url: String,
}

pub async fn handle_di_webhook_info(
    state: &AppState,
    input: &DiWebhookInfoInput,
) -> Result<Json<Value>, String> {
    ensure_discord_webhook_url(&input.webhook_url)?;
    let resp = state
        .media_http_client
        .get(&input.webhook_url)
        .send()
        .await
        .map_err(|e| format!("Webhook info failed: {e}"))?;
    let status = resp.status();
    let payload: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad webhook response: {e}"))?;
    if !status.is_success() {
        return Err(format!("Webhook not found or revoked ({status})"));
    }
    Ok(Json(payload))
}

#[cfg(test)]
mod webhook_tests {
    use super::*;

    #[test]
    fn discord_webhook_urls_are_host_guarded() {
        assert!(ensure_discord_webhook_url(
            "https://discord.com/api/webhooks/12345/abcdef"
        )
        .is_ok());
        assert!(ensure_discord_webhook_url(
            "https://discordapp.com/api/webhooks/12345/abcdef"
        )
        .is_ok());
        // A bare host would turn the tool into a post-anything gadget.
        assert!(ensure_discord_webhook_url("https://example.com/api/webhooks/1/x").is_err());
        assert!(ensure_discord_webhook_url("http://discord.com/api/webhooks/1/x").is_err());
        assert!(ensure_discord_webhook_url("https://discord.com/other/path").is_err());
        assert!(ensure_discord_webhook_url("not a url").is_err());
    }
}
