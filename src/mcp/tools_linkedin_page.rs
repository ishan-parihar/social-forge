// ─── MCP LinkedIn Page Tools ───────────────────────────────────
// LinkedIn Company Page tools — list pages, get page info,
// get page posts, and create comments as a page.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::linkedin_page::LinkedInPageProvider;
use crate::social::SocialProvider;

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipListPagesInput {
    pub user_id: Uuid,
    pub lip_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetPageInput {
    pub user_id: Uuid,
    pub lip_id: String,
    pub page_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LipGetPagePostsInput {
    pub user_id: Uuid,
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
    pub user_id: Uuid,
    pub lip_id: String,
    pub post_urn: String,
    pub page_urn: String,
    pub message: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_linkedin_page_token(
    state: &AppState,
    user_id: Uuid,
    lip_id: &str,
) -> Result<String, String> {
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

    Ok(integration.access_token.clone())
}

fn create_provider(state: &AppState) -> LinkedInPageProvider {
    LinkedInPageProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_lip_list_pages(
    state: &AppState,
    input: &LipListPagesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_page_token(state, user_id, &input.lip_id).await?;
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
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_page_token(state, user_id, &input.lip_id).await?;
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
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_page_token(state, user_id, &input.lip_id).await?;
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
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_page_token(state, user_id, &input.lip_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_comment(&token, &input.post_urn, &input.page_urn, &input.message)
        .await
        .map_err(|e| format!("LinkedIn Page create comment failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
