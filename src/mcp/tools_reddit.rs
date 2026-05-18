// ─── MCP Reddit Tools ───────────────────────────────────────────
// Reddit-specific read/query tools (browse, search, post detail, user info, DM, inbox).
// These call inherent methods on RedditProvider directly.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::reddit::RedditProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditGetCommentsInput {
    pub post_id: String,
    pub sort: Option<String>,
    pub depth: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditGetCommentsOutput {
    /// JSON array with 2 elements: [post_listing, comments_listing]
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditBrowseInput {
    pub subreddit: String,
    pub sort: Option<String>,
    pub limit: Option<u32>,
    pub time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditBrowseOutput {
    pub posts: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditSearchInput {
    pub query: String,
    pub subreddit: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<u32>,
    pub time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditSearchOutput {
    pub posts: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditPostDetailInput {
    pub post_id: String,
    pub depth: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditPostDetailOutput {
    /// JSON array with 2 elements: [post_listing, comments_listing]
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditUserInfoInput {
    pub username: String,
    pub include_posts: Option<bool>,
    pub include_comments: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditUserInfoOutput {
    pub user: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditSendDmInput {
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditSendDmOutput {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditInboxInput {
    pub folder: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditInboxOutput {
    pub messages: serde_json::Value,
}

// ── Helpers ──────────────────────────────────────────────────

/// Find the best Reddit token for the current user.
/// Priority: 1) DB cookie tokens 2) Browser extraction 3) OAuth DB tokens 4) Env vars
async fn find_reddit_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let reddit_integrations: Vec<_> = integrations
        .into_iter()
        .filter(|i| i.provider_identifier == "reddit")
        .collect();

    // Priority 1: DB-stored cookie tokens (JSON blobs with reddit_session)
    if let Some(preferred) = reddit_integrations.iter()
        .find(|i| i.access_token.starts_with('{'))
    {
        let token = preferred.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok(token);
    }

    // Priority 2: Browser cookie extraction
    if let Some(cookies) = crate::social::reddit_cookies::extract_reddit_cookies() {
        tracing::info!("Reddit cookies extracted from browser: {}", cookies.source);
        return Ok(crate::social::reddit_cookies::build_cookie_token(
            &cookies.reddit_session, cookies.token_v2.as_deref(), Some(&cookies.cookie_string)
        ));
    }

    // Priority 3: OAuth DB tokens
    if let Some(oauth) = reddit_integrations.first() {
        let token = oauth.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok(token);
    }

    Err("No Reddit integration found. Connect Reddit via /api/public/connect/reddit-cookies or integrations_connect.".into())
}

/// Create a RedditProvider from the app config (needed by MCP handlers).
fn create_provider(state: &AppState, token: &str) -> RedditProvider {
    let mut provider = RedditProvider::new(&state.config);
    provider.prepare_from_token(token);
    provider
}

// ── Tool Implementations ────────────────────────────────────

pub async fn reddit_get_comments(
    state: &AppState,
    input: &RedditGetCommentsInput,
) -> Result<Json<RedditGetCommentsOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let sort = input.sort.as_deref().unwrap_or("confidence");
    let depth = input.depth.unwrap_or(5);
    let limit = input.limit.unwrap_or(50);

    let result = provider
        .get_comments(&token, &input.post_id, sort, depth, limit)
        .await
        .map_err(|e| format!("Reddit get comments failed: {e}"))?;

    Ok(Json(RedditGetCommentsOutput { data: result }))
}

pub async fn reddit_browse(
    state: &AppState,
    input: &RedditBrowseInput,
) -> Result<Json<RedditBrowseOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let sort = input.sort.as_deref().unwrap_or("hot");
    let limit = input.limit.unwrap_or(25);
    let time = input.time.as_deref().unwrap_or("all");

    let result = provider
        .browse(&token, &input.subreddit, sort, limit, time)
        .await
        .map_err(|e| format!("Reddit browse failed: {e}"))?;

    Ok(Json(RedditBrowseOutput { posts: result }))
}

pub async fn reddit_search(
    state: &AppState,
    input: &RedditSearchInput,
) -> Result<Json<RedditSearchOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let sort = input.sort.as_deref().unwrap_or("relevance");
    let limit = input.limit.unwrap_or(25);
    let time = input.time.as_deref().unwrap_or("all");

    let result = provider
        .search(
            &token,
            &input.query,
            input.subreddit.as_deref(),
            sort,
            limit,
            time,
        )
        .await
        .map_err(|e| format!("Reddit search failed: {e}"))?;

    Ok(Json(RedditSearchOutput { posts: result }))
}

pub async fn reddit_post_detail(
    state: &AppState,
    input: &RedditPostDetailInput,
) -> Result<Json<RedditPostDetailOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let depth = input.depth.unwrap_or(5);
    let limit = input.limit.unwrap_or(50);

    let result = provider
        .post_detail(&token, &input.post_id, depth, limit)
        .await
        .map_err(|e| format!("Reddit post detail failed: {e}"))?;

    Ok(Json(RedditPostDetailOutput { data: result }))
}

pub async fn reddit_user_info(
    state: &AppState,
    input: &RedditUserInfoInput,
) -> Result<Json<RedditUserInfoOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let include_posts = input.include_posts.unwrap_or(true);
    let include_comments = input.include_comments.unwrap_or(false);

    let result = provider
        .user_info(&token, &input.username, include_posts, include_comments)
        .await
        .map_err(|e| format!("Reddit user info failed: {e}"))?;

    Ok(Json(RedditUserInfoOutput { user: result }))
}

pub async fn reddit_send_dm(
    state: &AppState,
    input: &RedditSendDmInput,
) -> Result<Json<RedditSendDmOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    provider
        .send_dm(&token, &input.to, &input.subject, &input.text)
        .await
        .map_err(|e| format!("Reddit DM failed: {e}"))?;

    Ok(Json(RedditSendDmOutput {
        success: true,
        message: format!("DM sent to u/{}", input.to),
    }))
}

pub async fn reddit_inbox(
    state: &AppState,
    input: &RedditInboxInput,
) -> Result<Json<RedditInboxOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let provider = create_provider(state, &token);

    let folder = input.folder.as_deref().unwrap_or("inbox");
    let limit = input.limit.unwrap_or(25);

    let result = provider
        .inbox(&token, folder, limit)
        .await
        .map_err(|e| format!("Reddit inbox failed: {e}"))?;

    Ok(Json(RedditInboxOutput { messages: result }))
}

// ─── Reddit Write Tools ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditCreatePostInput {
    pub subreddit: String,
    pub title: String,
    pub text: Option<String>,
    pub url: Option<String>,
    pub flair_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditCreatePostOutput {
    pub post_id: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditCreateCommentInput {
    /// Post ID (t3_xxx) or comment ID (t1_xxx) to reply to
    pub thing_id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditCreateCommentOutput {
    pub comment_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditGetKarmaOutput {
    pub data: serde_json::Value,
}

pub async fn handle_reddit_create_post(
    state: &AppState,
    input: &RedditCreatePostInput,
) -> Result<Json<RedditCreatePostOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;

    let subreddit = input.subreddit.replace("/r/", "").replace("r/", "");
    let kind = if input.url.is_some() { "link" } else { "self" };

    let mut form: Vec<(&str, &str)> = vec![
        ("api_type", "json"),
        ("sr", &subreddit),
        ("title", &input.title),
        ("kind", kind),
    ];
    let text_val = input.text.as_deref().unwrap_or("");
    let url_val = input.url.as_deref().unwrap_or("");
    if kind == "self" { form.push(("text", text_val)); }
    else { form.push(("url", url_val)); }
    if let Some(f) = &input.flair_id { form.push(("flair_id", f)); }

    let resp = reqwest::Client::new()
        .post("https://oauth.reddit.com/api/submit")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
        .form(&form).send().await
        .map_err(|e| format!("Reddit submit failed: {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {e}"))?;
    let post_url = json["json"]["data"]["url"].as_str().unwrap_or("").to_string();
    let post_id = json["json"]["data"]["id"].as_str().unwrap_or("").to_string();

    if post_id.is_empty() {
        return Err(format!("Reddit submit error: {}", serde_json::to_string(&json).unwrap_or_default()));
    }
    Ok(Json(RedditCreatePostOutput { post_id, url: post_url }))
}

pub async fn handle_reddit_create_comment(
    state: &AppState,
    input: &RedditCreateCommentInput,
) -> Result<Json<RedditCreateCommentOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;

    let thing_id = if input.thing_id.starts_with("t3_") || input.thing_id.starts_with("t1_") {
        input.thing_id.clone()
    } else {
        format!("t3_{}", input.thing_id)
    };

    let resp = reqwest::Client::new()
        .post("https://oauth.reddit.com/api/comment")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
        .form(&[("api_type", "json"), ("thing_id", &thing_id), ("text", &input.text)])
        .send().await
        .map_err(|e| format!("Reddit comment failed: {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {e}"))?;
    let comment_id = json["json"]["data"]["things"][0]["data"]["id"].as_str().unwrap_or("").to_string();

    if comment_id.is_empty() {
        return Err(format!("Reddit comment error: {}", serde_json::to_string(&json).unwrap_or_default()));
    }
    Ok(Json(RedditCreateCommentOutput { comment_id }))
}

pub async fn handle_reddit_get_karma(
    state: &AppState,
) -> Result<Json<RedditGetKarmaOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;

    let resp = reqwest::Client::new()
        .get("https://oauth.reddit.com/api/v1/me/karma")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
        .send().await
        .map_err(|e| format!("Reddit karma failed: {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {e}"))?;
    Ok(Json(RedditGetKarmaOutput { data: json }))
}

// ─── Reddit Cookie-Based Write Tools ────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditVoteInput {
    /// Thing ID (t3_xxx for post, t1_xxx for comment)
    pub thing_id: String,
    /// 1 = upvote, 0 = unvote, -1 = downvote
    pub direction: i8,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditThingInput {
    /// Thing ID (t3_xxx for post, t1_xxx for comment)
    pub thing_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditSubscribeInput {
    pub subreddit: String,
    /// "sub" to subscribe, "unsub" to unsubscribe
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditEditInput {
    /// Thing ID of the post/comment to edit
    pub thing_id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditModRemoveInput {
    pub thing_id: String,
    /// true to mark as spam, false for regular removal
    pub spam: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditModDistinguishInput {
    pub thing_id: String,
    /// "yes", "no", "admin", or "special"
    pub how: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditModStickyInput {
    pub thing_id: String,
    /// true to sticky, false to unsticky
    pub state: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RedditActionOutput {
    pub success: bool,
}

pub async fn handle_reddit_vote(
    state: &AppState,
    input: &RedditVoteInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.vote(&input.thing_id, input.direction).await
        .map_err(|e| format!("Reddit vote failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_save(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.save(&input.thing_id).await
        .map_err(|e| format!("Reddit save failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_unsave(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.unsave(&input.thing_id).await
        .map_err(|e| format!("Reddit unsave failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_hide(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.hide(&input.thing_id).await
        .map_err(|e| format!("Reddit hide failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_subscribe(
    state: &AppState,
    input: &RedditSubscribeInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.subscribe(&input.subreddit, &input.action).await
        .map_err(|e| format!("Reddit subscribe failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_edit(
    state: &AppState,
    input: &RedditEditInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.edit_text(&input.thing_id, &input.text).await
        .map_err(|e| format!("Reddit edit failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_delete(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.delete(&input.thing_id).await
        .map_err(|e| format!("Reddit delete failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_mod_remove(
    state: &AppState,
    input: &RedditModRemoveInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.mod_remove(&input.thing_id, input.spam.unwrap_or(false)).await
        .map_err(|e| format!("Reddit mod remove failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_mod_approve(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.mod_approve(&input.thing_id).await
        .map_err(|e| format!("Reddit mod approve failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_mod_distinguish(
    state: &AppState,
    input: &RedditModDistinguishInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.mod_distinguish(&input.thing_id, &input.how).await
        .map_err(|e| format!("Reddit mod distinguish failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_mod_sticky(
    state: &AppState,
    input: &RedditModStickyInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.mod_sticky(&input.thing_id, input.state).await
        .map_err(|e| format!("Reddit mod sticky failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_mod_lock(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.mod_lock(&input.thing_id).await
        .map_err(|e| format!("Reddit mod lock failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}

pub async fn handle_reddit_mod_unlock(
    state: &AppState,
    input: &RedditThingInput,
) -> Result<Json<RedditActionOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;
    let mut provider = create_provider(state, &token);
    provider.mod_unlock(&input.thing_id).await
        .map_err(|e| format!("Reddit mod unlock failed: {e}"))?;
    Ok(Json(RedditActionOutput { success: true }))
}
