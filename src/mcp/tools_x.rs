// ─── MCP X/Twitter Tools ─────────────────────────────────────────
// X-specific read/write tools via Twitter API v2 using OAuth 2.0 Bearer tokens.
// Follows the same pattern as tools_reddit.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::x::XProvider;

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

async fn resolve_first_user(state: &AppState) -> Result<Uuid, String> {
    super::tools_posts::resolve_first_user(state).await
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
