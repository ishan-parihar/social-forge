// ─── MCP X/Twitter Tools ─────────────────────────────────────────
// X-specific read/write tools via Twitter API v2 using OAuth 2.0 Bearer tokens.
// Follows the same pattern as tools_reddit.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::x::XProvider;
use crate::social::SocialProvider;
use super::auth::resolve_first_user;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetMeOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XHomeTimelineInput {
    pub max_results: Option<u32>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XHomeTimelineOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUserLookupInput {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUserLookupOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUserLookupByUsernameInput {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUserLookupByUsernameOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUserTweetsInput {
    pub user_id: String,
    pub max_results: Option<u32>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUserTweetsOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XTweetDetailInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XTweetDetailOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XSearchTweetsInput {
    pub query: String,
    pub max_results: Option<u32>,
    pub next_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XSearchTweetsOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XDeleteTweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XDeleteTweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XLikeTweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XLikeTweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnlikeTweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnlikeTweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XRetweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XRetweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnretweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnretweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XBookmarksInput {
    pub max_results: Option<u32>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XBookmarksOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XBookmarkTweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XBookmarkTweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnbookmarkTweetInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnbookmarkTweetOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XFollowersInput {
    pub user_id: String,
    pub max_results: Option<u32>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XFollowersOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XFollowingInput {
    pub user_id: String,
    pub max_results: Option<u32>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XFollowingOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XFollowUserInput {
    pub target_user_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XFollowUserOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnfollowUserInput {
    pub target_user_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XUnfollowUserOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XListTimelineInput {
    pub list_id: String,
    pub max_results: Option<u32>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XListTimelineOutput {
    pub data: serde_json::Value,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_x_token(state: &AppState, user_id: Uuid) -> Result<(String, String), String> {
    // Priority 1: DB-stored cookie tokens (freshest — submitted via web form)
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let x_integrations: Vec<_> = integrations
        .into_iter()
        .filter(|i| i.provider_identifier == "x")
        .collect();

    if let Some(preferred) = x_integrations.iter()
        .find(|i| i.access_token.starts_with('{'))
    {
        let token = preferred.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok((token, preferred.internal_id.clone()));
    }

    // Priority 2: Env vars (X_AUTH_TOKEN + X_CT0) — stale but quick
    if let (Some(auth_token), Some(ct0)) = (&state.config.x_auth_token, &state.config.x_ct0) {
        let token = serde_json::json!({
            "auth_token": auth_token,
            "ct0": ct0,
        })
        .to_string();
        return Ok((token, String::new()));
    }

    // Priority 3: Browser cookie extraction (Chrome/Brave/Firefox)
    if let Some(cookies) = crate::social::x_cookies::extract_x_cookies() {
        tracing::info!("X cookies extracted from browser: {}", cookies.source);
        let token = crate::social::x_cookies::build_cookie_token(
            &cookies.auth_token, &cookies.ct0, Some(&cookies.cookie_string)
        );
        return Ok((token, String::new()));
    }

    // Priority 4: OAuth DB tokens as last resort
    if let Some(oauth) = x_integrations.first() {
        let token = oauth.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok((token, oauth.internal_id.clone()));
    }

    Err("No X/Twitter integration found. Enter fresh cookies at /api/public/connect/x-cookies, set X_AUTH_TOKEN + X_CT0 env vars, or connect via OAuth.".into())
}

fn create_provider(state: &AppState, token: &str) -> XProvider {
    let mut provider = XProvider::new(&state.config);
    provider.prepare_from_token(token);
    provider
}



// ── Tool Implementations ─────────────────────────────────────

pub async fn x_get_me(
    state: &AppState,
) -> Result<Json<XGetMeOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider.get_me(&token).await.map_err(|e| format!("X get_me failed: {e}"))?;
    Ok(Json(XGetMeOutput { data: result }))
}

pub async fn x_home_timeline(
    state: &AppState,
    input: &XHomeTimelineInput,
) -> Result<Json<XHomeTimelineOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .home_timeline(&token, &my_id, max_results, input.pagination_token.as_deref())
        .await
        .map_err(|e| format!("X home timeline failed: {e}"))?;
    Ok(Json(XHomeTimelineOutput { data: result }))
}

pub async fn x_user_lookup(
    state: &AppState,
    input: &XUserLookupInput,
) -> Result<Json<XUserLookupOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .user_lookup(&token, &input.user_id)
        .await
        .map_err(|e| format!("X user lookup failed: {e}"))?;
    Ok(Json(XUserLookupOutput { data: result }))
}

pub async fn x_user_lookup_by_username(
    state: &AppState,
    input: &XUserLookupByUsernameInput,
) -> Result<Json<XUserLookupByUsernameOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .user_lookup_by_username(&token, &input.username)
        .await
        .map_err(|e| format!("X user lookup by username failed: {e}"))?;
    Ok(Json(XUserLookupByUsernameOutput { data: result }))
}

pub async fn x_user_tweets(
    state: &AppState,
    input: &XUserTweetsInput,
) -> Result<Json<XUserTweetsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .user_tweets(&token, &input.user_id, max_results, input.pagination_token.as_deref())
        .await
        .map_err(|e| format!("X user tweets failed: {e}"))?;
    Ok(Json(XUserTweetsOutput { data: result }))
}

pub async fn x_tweet_detail(
    state: &AppState,
    input: &XTweetDetailInput,
) -> Result<Json<XTweetDetailOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .tweet_detail(&token, &input.tweet_id)
        .await
        .map_err(|e| format!("X tweet detail failed: {e}"))?;
    Ok(Json(XTweetDetailOutput { data: result }))
}

pub async fn x_search_tweets(
    state: &AppState,
    input: &XSearchTweetsInput,
) -> Result<Json<XSearchTweetsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .search_tweets(&token, &input.query, max_results, input.next_token.as_deref())
        .await
        .map_err(|e| format!("X search failed: {e}"))?;
    Ok(Json(XSearchTweetsOutput { data: result }))
}

pub async fn x_delete_tweet(
    state: &AppState,
    input: &XDeleteTweetInput,
) -> Result<Json<XDeleteTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .delete_tweet(&token, &input.tweet_id)
        .await
        .map_err(|e| format!("X delete tweet failed: {e}"))?;
    Ok(Json(XDeleteTweetOutput { data: result }))
}

pub async fn x_like_tweet(
    state: &AppState,
    input: &XLikeTweetInput,
) -> Result<Json<XLikeTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .like_tweet(&token, &my_id, &input.tweet_id)
        .await
        .map_err(|e| format!("X like failed: {e}"))?;
    Ok(Json(XLikeTweetOutput { data: result }))
}

pub async fn x_unlike_tweet(
    state: &AppState,
    input: &XUnlikeTweetInput,
) -> Result<Json<XUnlikeTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .unlike_tweet(&token, &my_id, &input.tweet_id)
        .await
        .map_err(|e| format!("X unlike failed: {e}"))?;
    Ok(Json(XUnlikeTweetOutput { data: result }))
}

pub async fn x_retweet(
    state: &AppState,
    input: &XRetweetInput,
) -> Result<Json<XRetweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .retweet(&token, &my_id, &input.tweet_id)
        .await
        .map_err(|e| format!("X retweet failed: {e}"))?;
    Ok(Json(XRetweetOutput { data: result }))
}

pub async fn x_unretweet(
    state: &AppState,
    input: &XUnretweetInput,
) -> Result<Json<XUnretweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .unretweet(&token, &my_id, &input.tweet_id)
        .await
        .map_err(|e| format!("X unretweet failed: {e}"))?;
    Ok(Json(XUnretweetOutput { data: result }))
}

pub async fn x_bookmarks(
    state: &AppState,
    input: &XBookmarksInput,
) -> Result<Json<XBookmarksOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .bookmarks(&token, &my_id, max_results, input.pagination_token.as_deref())
        .await
        .map_err(|e| format!("X bookmarks failed: {e}"))?;
    Ok(Json(XBookmarksOutput { data: result }))
}

pub async fn x_bookmark_tweet(
    state: &AppState,
    input: &XBookmarkTweetInput,
) -> Result<Json<XBookmarkTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .bookmark_tweet(&token, &my_id, &input.tweet_id)
        .await
        .map_err(|e| format!("X bookmark failed: {e}"))?;
    Ok(Json(XBookmarkTweetOutput { data: result }))
}

pub async fn x_unbookmark_tweet(
    state: &AppState,
    input: &XUnbookmarkTweetInput,
) -> Result<Json<XUnbookmarkTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .unbookmark_tweet(&token, &my_id, &input.tweet_id)
        .await
        .map_err(|e| format!("X unbookmark failed: {e}"))?;
    Ok(Json(XUnbookmarkTweetOutput { data: result }))
}

pub async fn x_followers(
    state: &AppState,
    input: &XFollowersInput,
) -> Result<Json<XFollowersOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .followers(&token, &input.user_id, max_results, input.pagination_token.as_deref())
        .await
        .map_err(|e| format!("X followers failed: {e}"))?;
    Ok(Json(XFollowersOutput { data: result }))
}

pub async fn x_following(
    state: &AppState,
    input: &XFollowingInput,
) -> Result<Json<XFollowingOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .following(&token, &input.user_id, max_results, input.pagination_token.as_deref())
        .await
        .map_err(|e| format!("X following failed: {e}"))?;
    Ok(Json(XFollowingOutput { data: result }))
}

pub async fn x_follow_user(
    state: &AppState,
    input: &XFollowUserInput,
) -> Result<Json<XFollowUserOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .follow_user(&token, &my_id, &input.target_user_id)
        .await
        .map_err(|e| format!("X follow failed: {e}"))?;
    Ok(Json(XFollowUserOutput { data: result }))
}

pub async fn x_unfollow_user(
    state: &AppState,
    input: &XUnfollowUserInput,
) -> Result<Json<XUnfollowUserOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let result = provider
        .unfollow_user(&token, &my_id, &input.target_user_id)
        .await
        .map_err(|e| format!("X unfollow failed: {e}"))?;
    Ok(Json(XUnfollowUserOutput { data: result }))
}

pub async fn x_list_timeline(
    state: &AppState,
    input: &XListTimelineInput,
) -> Result<Json<XListTimelineOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let max_results = input.max_results.unwrap_or(20).min(100);
    let result = provider
        .list_timeline(&token, &input.list_id, max_results, input.pagination_token.as_deref())
        .await
        .map_err(|e| format!("X list timeline failed: {e}"))?;
    Ok(Json(XListTimelineOutput { data: result }))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XReplyTweetInput {
    pub tweet_id: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XReplyTweetOutput {
    pub tweet_id: String,
    pub url: Option<String>,
}

// ── Create Tweet ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XCreateTweetInput {
    /// The text content of the tweet (max 280 chars for standard, 4000 for premium)
    pub content: String,
    /// Optional media URLs (images/videos) to attach. Use media_upload first to get URLs.
    #[serde(default)]
    pub media_urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XCreateTweetOutput {
    pub tweet_id: String,
    pub url: Option<String>,
    pub status: String,
}

/// Create and immediately publish a new tweet on X/Twitter.
/// For media tweets, first upload via media_upload_from_path or media_upload,
/// then pass the returned URLs in media_urls.
pub async fn x_create_tweet(
    state: &AppState,
    input: &XCreateTweetInput,
) -> Result<Json<XCreateTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let media: Vec<crate::social::MediaAttachment> = input
        .media_urls
        .iter()
        .map(|url| crate::social::MediaAttachment {
            url: url.clone(),
            mime_type: "image/jpeg".to_string(), // X auto-detects from URL
            alt: None,
            poster_url: None,
        })
        .collect();

    let post_content = crate::social::PostContent {
        content: input.content.clone(),
        media,
        settings: serde_json::json!({}),
    };

    let result = provider
        .publish(&token, &post_content)
        .await
        .map_err(|e| format!("Failed to create tweet: {e}"))?;

    Ok(Json(XCreateTweetOutput {
        tweet_id: result.platform_post_id,
        url: result.platform_post_url,
        status: result.status,
    }))
}

pub async fn x_reply_tweet(
    state: &AppState,
    input: &XReplyTweetInput,
) -> Result<Json<XReplyTweetOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let post_content = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    };
    let result = provider
        .reply_to_comment(&token, &input.tweet_id, &post_content)
        .await
        .map_err(|e| format!("X reply failed: {e}"))?;
    Ok(Json(XReplyTweetOutput {
        tweet_id: result.platform_post_id,
        url: result.platform_post_url,
    }))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XSendDmInput {
    pub recipient_id: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XSendDmOutput {
    pub message_id: String,
    pub status: String,
}

pub async fn x_send_dm(
    state: &AppState,
    input: &XSendDmInput,
) -> Result<Json<XSendDmOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let post_content = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    };
    let result = provider
        .send_dm(&token, &input.recipient_id, &post_content)
        .await
        .map_err(|e| format!("X send DM failed: {e}"))?;
    Ok(Json(XSendDmOutput {
        message_id: result.platform_post_id,
        status: result.status,
    }))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XListDmsInput {
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XListDmsOutput {
    pub conversations: Vec<serde_json::Value>,
}

pub async fn x_list_dms(
    state: &AppState,
    input: &XListDmsInput,
) -> Result<Json<XListDmsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let limit = input.max_results.unwrap_or(20).min(50);
    let conversations = provider
        .get_dm_conversations(&token, limit)
        .await
        .map_err(|e| format!("X list DMs failed: {e}"))?;
    let conv_values: Vec<serde_json::Value> = conversations.into_iter().map(|c| {
        serde_json::json!({
            "id": c.id,
            "participant": c.participant,
            "last_message": c.last_message,
            "last_message_at": c.last_message_at.map(|dt| dt.to_rfc3339()),
        })
    }).collect();
    Ok(Json(XListDmsOutput { conversations: conv_values }))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetDmConversationInput {
    pub conversation_id: String,
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetDmConversationOutput {
    pub messages: Vec<serde_json::Value>,
}

pub async fn x_get_dm_conversation(
    state: &AppState,
    input: &XGetDmConversationInput,
) -> Result<Json<XGetDmConversationOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);
    let limit = input.max_results.unwrap_or(20).min(50);
    let messages = provider
        .get_dm_messages(&token, &input.conversation_id, limit)
        .await
        .map_err(|e| format!("X get DM conversation failed: {e}"))?;
    let msg_values: Vec<serde_json::Value> = messages.into_iter().map(|m| {
        serde_json::json!({
            "id": m.id,
            "sender": m.sender,
            "content": m.content,
            "created_at": m.created_at.to_rfc3339(),
        })
    }).collect();
    Ok(Json(XGetDmConversationOutput { messages: msg_values }))
}

// ── Analytics ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetAnalyticsInput {
    /// Number of days of analytics to retrieve (default 30)
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_days() -> u32 { 30 }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetAnalyticsOutput {
    pub data: serde_json::Value,
}

/// Get account-level analytics for the authenticated X/Twitter user
/// (followers, following, tweet count, etc.)
pub async fn x_get_analytics(
    state: &AppState,
    input: &XGetAnalyticsInput,
) -> Result<Json<XGetAnalyticsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let analytics = provider
        .analytics(&token, &my_id, input.days)
        .await
        .map_err(|e| format!("X analytics failed: {e}"))?;

    Ok(Json(XGetAnalyticsOutput {
        data: serde_json::to_value(&analytics).unwrap_or(serde_json::Value::Null),
    }))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetPostAnalyticsInput {
    pub tweet_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XGetPostAnalyticsOutput {
    pub data: serde_json::Value,
}

/// Get engagement metrics (likes, retweets, replies, impressions) for a specific tweet.
pub async fn x_get_post_analytics(
    state: &AppState,
    input: &XGetPostAnalyticsInput,
) -> Result<Json<XGetPostAnalyticsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let (token, _) = find_x_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let analytics = provider
        .post_analytics(&token, &input.tweet_id)
        .await
        .map_err(|e| format!("X post analytics failed: {e}"))?;

    Ok(Json(XGetPostAnalyticsOutput {
        data: serde_json::to_value(&analytics).unwrap_or(serde_json::Value::Null),
    }))
}
