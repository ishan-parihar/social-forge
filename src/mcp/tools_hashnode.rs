// ─── MCP Hashnode Tools ────────────────────────────────────────────
// Hashnode API tools via the HashnodeProvider (API key-based).

use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;
use crate::social::hashnode::HashnodeProvider;

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct HnCreatePostInput {
    /// Hashnode publication ID
    pub publication_id: String,
    /// Post title
    pub title: String,
    /// Post body content (markdown)
    pub content: String,
    /// Tags as JSON array of { _id, slug, name }
    pub tags: Option<String>,
    /// Canonical URL (optional)
    pub canonical_url: Option<String>,
    /// §5.4: which connected Hashnode account to use — integration UUID,
    /// "hashnode:Name", or empty for the first connected account.
    #[serde(default)]
    pub integration: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct HnListPostsInput {
    /// Hashnode publication ID
    pub publication_id: String,
    /// Page number (default: 0)
    pub page: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct HnGetPostInput {
    /// Hashnode post ID
    pub post_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_hashnode_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    integrations
        .into_iter()
        .find(|i| i.provider_identifier == "hashnode")
        .map(|i| i.access_token)
        .ok_or_else(|| {
            "No Hashnode integration found. Connect Hashnode first via integrations_connect."
                .to_string()
        })
}

fn create_hashnode_provider(state: &AppState) -> HashnodeProvider {
    HashnodeProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_hn_create_post(
    state: &AppState,
    input: &HnCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    // §4.2: through the publish pipeline — posts row + idempotency + retries.
    let integration = super::tools_posts::select_integration(
        state,
        user_id,
        "hashnode",
        input.integration.as_deref(),
    )
    .await?;

    let mut settings = json!({
        "publication_id": input.publication_id,
        "title": input.title,
    });

    if let Some(tags_str) = &input.tags {
        if let Ok(tags_val) = serde_json::from_str::<serde_json::Value>(tags_str) {
            settings["tags"] = tags_val;
        }
    }

    if let Some(canonical_url) = &input.canonical_url {
        settings["canonical_url"] = json!(canonical_url);
    }

    let (_post_id, platform_id, url) = super::tools_posts::publish_now_via_pipeline(
        state,
        &integration,
        &input.content,
        Some(&input.title),
        vec![],
        settings,
        None,
    )
    .await?;

    Ok(Json(json!({
        "platform_post_id": platform_id,
        "platform_post_url": if url.is_empty() { None } else { Some(url) },
        "status": "published",
    })))
}

pub async fn handle_hn_list_posts(
    state: &AppState,
    input: &HnListPostsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_hashnode_token(state, user_id).await?;
    let provider = create_hashnode_provider(state);

    let page = input.page.unwrap_or(0);

    let posts = provider
        .list_posts(&token, &input.publication_id, page)
        .await
        .map_err(|e| format!("Hashnode list posts failed: {e}"))?;

    Ok(Json(posts))
}

pub async fn handle_hn_get_post(
    state: &AppState,
    input: &HnGetPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_hashnode_token(state, user_id).await?;
    let provider = create_hashnode_provider(state);

    let post = provider
        .get_post(&token, &input.post_id)
        .await
        .map_err(|e| format!("Hashnode get post failed: {e}"))?;

    Ok(Json(post))
}
