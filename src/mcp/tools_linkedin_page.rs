// ─── MCP LinkedIn Page Tools ───────────────────────────────────
// LinkedIn Company Page tools — list pages, get page info,
// get page posts, and create comments as a page.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::social::linkedin_page::LinkedInPageProvider;
use crate::social::SocialProvider;

// ── Input Types ──────────────────────────────────────────────
//
// Note: no `user_id` field on any input. Single-user mode means
// `resolve_first_user` returns `DEFAULT_USER_ID` for every call.
// Letting callers supply a UUID was an auth-bypass bug.

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipListPagesInput {
    pub lip_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetPageInput {
    pub lip_id: String,
    pub page_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetPagePostsInput {
    pub lip_id: String,
    pub page_id: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipCreateCommentInput {
    pub lip_id: String,
    pub post_urn: String,
    pub page_urn: String,
    pub message: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_linkedin_page_token(
    state: &AppState,
    lip_id: &str,
) -> Result<String, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == "linkedin-page" && i.internal_id == lip_id)
        .ok_or_else(|| {
            format!(
                "LinkedIn Page account '{}' not connected. Connect it via the onboarding page first.",
                lip_id
            )
        })?;

    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
    Ok(tok)
}

fn create_provider(state: &AppState) -> LinkedInPageProvider {
    LinkedInPageProvider::new(&state.config)
}

/// LinkedIn REST API version header — keep this pinned to a real
/// released version (NOT a future-dated one). LinkedIn rejects
/// versions newer than the current month.
const LINKEDIN_VERSION: &str = "202401";

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_lip_list_pages(
    state: &AppState,
    input: &LipListPagesInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let provider = create_provider(state);
    let result = provider
        .pages(&token)
        .await
        .map_err(|e| format!("LinkedIn Page list pages failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_lip_get_page(
    state: &AppState,
    input: &LipGetPageInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let provider = create_provider(state);
    let result = provider
        .fetch_page_info(&token, &input.page_id)
        .await
        .map_err(|e| format!("LinkedIn Page get page failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_lip_get_page_posts(
    state: &AppState,
    input: &LipGetPagePostsInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_page_posts(&token, &input.page_id, input.limit)
        .await
        .map_err(|e| format!("LinkedIn Page get page posts failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_lip_create_comment(
    state: &AppState,
    input: &LipCreateCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_comment(&token, &input.post_urn, &input.page_urn, &input.message)
        .await
        .map_err(|e| format!("LinkedIn Page create comment failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

// ─── LinkedIn Page Write/Analytics Tools ─────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipCreatePostInput {
    pub lip_id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipCreatePostOutput {
    pub post_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetAnalyticsInput {
    pub lip_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetAnalyticsOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetPostAnalyticsInput {
    pub lip_id: String,
    pub post_urn: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetFollowersInput {
    pub lip_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetFollowersOutput {
    pub follower_count: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipDeletePostInput {
    pub lip_id: String,
    pub post_urn: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetReactionsInput {
    pub lip_id: String,
    pub post_urn: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetSharesInput {
    pub lip_id: String,
    pub post_urn: String,
}

pub async fn handle_lip_create_post(
    state: &AppState,
    input: &LipCreatePostInput,
) -> Result<Json<LipCreatePostOutput>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;

    let body = serde_json::json!({
        "author": format!("urn:li:organization:{}", input.lip_id),
        "commentary": input.text,
        "visibility": "PUBLIC",
        "distribution": { "feedDistribution": "MAIN_FEED" },
        "lifecycleState": "PUBLISHED",
    });

    let resp = reqwest::Client::new()
        .post("https://api.linkedin.com/v2/rest/posts")
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Restli-Protocol-Version", "2.0.0")
        .header("LinkedIn-Version", LINKEDIN_VERSION)
        .header("Content-Type", "application/json")
        .json(&body).send().await
        .map_err(|e| format!("LinkedIn post failed: {e}"))?;

    let status = resp.status();
    let post_id = resp.headers().get("x-restli-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("").to_string();

    if !status.is_success() {
        return Err(format!("LinkedIn post failed ({})", status));
    }
    Ok(Json(LipCreatePostOutput { post_id }))
}

pub async fn handle_lip_get_analytics(
    state: &AppState,
    input: &LipGetAnalyticsInput,
) -> Result<Json<LipGetAnalyticsOutput>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let provider = create_provider(state);
    let data = provider.analytics(&token, &input.lip_id, 30).await
        .map_err(|e| format!("LinkedIn analytics failed: {e}"))?;
    Ok(Json(LipGetAnalyticsOutput { data: serde_json::to_value(data).unwrap_or_default() }))
}

pub async fn handle_lip_get_post_analytics(
    state: &AppState,
    input: &LipGetPostAnalyticsInput,
) -> Result<Json<LipGetAnalyticsOutput>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let provider = create_provider(state);
    let data = provider.post_analytics(&token, &input.post_urn).await
        .map_err(|e| format!("LinkedIn post analytics failed: {e}"))?;
    Ok(Json(LipGetAnalyticsOutput { data: serde_json::to_value(data).unwrap_or_default() }))
}

pub async fn handle_lip_get_followers(
    state: &AppState,
    input: &LipGetFollowersInput,
) -> Result<Json<LipGetFollowersOutput>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let org_urn = format!("urn:li:organization:{}", input.lip_id);
    let url = format!(
        "https://api.linkedin.com/rest/networkSizes/{org_urn}?edgeType=CompanyFollowedByMember"
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("LinkedIn-Version", LINKEDIN_VERSION)
        .header("X-Restli-Protocol-Version", "2.0.0")
        .send().await
        .map_err(|e| format!("LinkedIn followers failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("LinkedIn followers failed ({})", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
    let count = json["firstDegreeSize"].as_u64().unwrap_or(0);
    Ok(Json(LipGetFollowersOutput { follower_count: count }))
}

pub async fn handle_lip_delete_post(
    state: &AppState,
    input: &LipDeletePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let url = format!(
        "https://api.linkedin.com/v2/rest/posts/{}",
        urlencoding::encode(&input.post_urn)
    );
    let resp = reqwest::Client::new()
        .delete(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Restli-Protocol-Version", "2.0.0")
        .header("LinkedIn-Version", LINKEDIN_VERSION)
        .send().await
        .map_err(|e| format!("LinkedIn delete failed: {e}"))?;

    let status = resp.status();
    if status.is_success() || status.as_u16() == 204 {
        Ok(Json(serde_json::json!({"deleted": true, "post_urn": input.post_urn})))
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("LinkedIn Page delete failed ({}): {}", status, body))
    }
}

pub async fn handle_lip_get_reactions(
    state: &AppState,
    input: &LipGetReactionsInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let url = format!(
        "https://api.linkedin.com/v2/rest/socialActions/{}/likes",
        input.post_urn
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Restli-Protocol-Version", "2.0.0")
        .header("LinkedIn-Version", LINKEDIN_VERSION)
        .send().await
        .map_err(|e| format!("LinkedIn get reactions failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("LinkedIn get reactions failed ({})", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
    Ok(Json(serde_json::json!({ "data": json })))
}

pub async fn handle_lip_get_shares(
    state: &AppState,
    input: &LipGetSharesInput,
) -> Result<Json<serde_json::Value>, String> {
    let token = find_linkedin_page_token(state, &input.lip_id).await?;
    let url = format!(
        "https://api.linkedin.com/v2/rest/socialActions/{}/shares",
        input.post_urn
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Restli-Protocol-Version", "2.0.0")
        .header("LinkedIn-Version", LINKEDIN_VERSION)
        .send().await
        .map_err(|e| format!("LinkedIn get shares failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("LinkedIn get shares failed ({})", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
    Ok(Json(serde_json::json!({ "data": json })))
}
