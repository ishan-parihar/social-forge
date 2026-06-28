// ─── MCP Bluesky Tools ──────────────────────────────────────────
// Bluesky AT Protocol tools (profile, timeline, create_post, search, feed).
// Follows the same pattern as tools_youtube.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BsProfileInput {
    pub token: String,
    pub handle_or_did: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BsTimelineInput {
    pub token: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BsCreatePostInput {
    pub token: String,
    pub text: String,
    pub image_urls: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BsSearchInput {
    pub token: String,
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BsFeedInput {
    pub token: String,
    pub feed_type: Option<String>,
    pub limit: Option<u32>,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_bs_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let bs = integrations
        .iter()
        .find(|i| i.provider_identifier == "bluesky")
        .ok_or_else(|| {
            "Bluesky account not connected. Connect it via the onboarding page first.".to_string()
        })?;

    let __tok = bs.access_token.clone();
    let __tok = state.token_key.as_ref()
        .and_then(|k| crate::crypto::decrypt_string(&__tok, k).ok())
        .unwrap_or(__tok);
    Ok(__tok)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_bs_profile(
    state: &AppState,
    input: &BsProfileInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_bs_token(state, user_id).await?;
    let client = reqwest::Client::new();

    let resp = client
        .get("https://bsky.social/xrpc/app.bsky.actor.getProfile")
        .header("Authorization", format!("Bearer {token}"))
        .query(&[("actor", &input.handle_or_did)])
        .send()
        .await
        .map_err(|e| format!("Bluesky profile request failed: {e}"))?;

    let result: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bluesky profile response: {e}"))?;

    Ok(Json(json!({ "data": result })))
}

pub async fn handle_bs_timeline(
    state: &AppState,
    input: &BsTimelineInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_bs_token(state, user_id).await?;
    let client = reqwest::Client::new();

    let limit = input.limit.unwrap_or(20);
    let mut query_params = vec![("limit", limit.to_string())];
    if let Some(cursor) = &input.cursor {
        query_params.push(("cursor", cursor.clone()));
    }

    let resp = client
        .get("https://bsky.social/xrpc/app.bsky.feed.getTimeline")
        .header("Authorization", format!("Bearer {token}"))
        .query(&query_params)
        .send()
        .await
        .map_err(|e| format!("Bluesky timeline request failed: {e}"))?;

    let result: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bluesky timeline response: {e}"))?;

    Ok(Json(json!({ "data": result })))
}

pub async fn handle_bs_create_post(
    state: &AppState,
    input: &BsCreatePostInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_bs_token(state, user_id).await?;

    let provider = state
        .providers
        .get("bluesky")
        .ok_or_else(|| "Bluesky provider not found in registry".to_string())?;

    let media = if let Some(urls) = &input.image_urls {
        urls.iter()
            .map(|url| crate::social::MediaAttachment {
                url: url.clone(),
                mime_type: "image/jpeg".to_string(),
                alt: None,
                poster_url: None,
            })
            .collect()
    } else {
        vec![]
    };

    let post_content = crate::social::PostContent {
        content: input.text.clone(),
        media,
        settings: json!({}),
    };

    let result = provider
        .publish(&token, &post_content)
        .await
        .map_err(|e| format!("Bluesky publish failed: {e}"))?;

    Ok(Json(json!({ "data": {
        "uri": result.platform_post_url,
        "cid": result.platform_post_id,
        "status": result.status,
    }})))
}

pub async fn handle_bs_search(
    state: &AppState,
    input: &BsSearchInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_bs_token(state, user_id).await?;
    let client = reqwest::Client::new();

    let limit = input.limit.unwrap_or(10);

    let resp = client
        .get("https://bsky.social/xrpc/app.bsky.actor.searchActors")
        .header("Authorization", format!("Bearer {token}"))
        .query(&[("q", &input.query), ("limit", &limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("Bluesky search request failed: {e}"))?;

    let result: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bluesky search response: {e}"))?;

    Ok(Json(json!({ "data": result })))
}

pub async fn handle_bs_feed(
    state: &AppState,
    input: &BsFeedInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_bs_token(state, user_id).await?;
    let client = reqwest::Client::new();

    let limit = input.limit.unwrap_or(20);

    // Use getTimeline for the default feed (feed_type is reserved for future use
    // with custom feeds like what's hot, etc.)
    let url = match input.feed_type.as_deref() {
        Some("popular") | Some("whats-hot") => {
            "https://bsky.social/xrpc/app.bsky.feed.getFeed?feed=at://did:plc:z72i7hd2gqrxv5f6z6q3y6z6/app.bsky.feed.generator/whats-hot"
        }
        _ => "https://bsky.social/xrpc/app.bsky.feed.getTimeline",
    };

    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .query(&[("limit", &limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("Bluesky feed request failed: {e}"))?;

    let result: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bluesky feed response: {e}"))?;

    Ok(Json(json!({ "data": result })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BsReplyInput {
    pub token: String,
    pub post_uri: String,
    pub content: String,
}

pub async fn handle_bs_reply(
    state: &AppState,
    input: &BsReplyInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_bs_token(state, user_id).await?;
    let client = reqwest::Client::new();

    let body = json!({
        "collection": "app.bsky.feed.post",
        "repo": input.token,
        "record": {
            "$type": "app.bsky.feed.post",
            "text": input.content,
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "reply": {
                "root": {
                    "uri": input.post_uri,
                    "cid": ""
                },
                "parent": {
                    "uri": input.post_uri,
                    "cid": ""
                }
            }
        }
    });

    let resp = client
        .post("https://bsky.social/xrpc/com.atproto.repo.createRecord")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Bluesky reply request failed: {e}"))?;

    let result: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bluesky reply response: {e}"))?;

    Ok(Json(json!({ "data": result })))
}
