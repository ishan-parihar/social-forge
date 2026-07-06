// ─── Music API Routes ────────────────────────────────────────
// Instagram Audio API integration — search trending music and
// original sounds, attach to Reels at publish time.
//
// Based on Instagram Graph API v22.0+ Audio API:
//   GET /ig_audio?audio_type=music&search_query=...
//   POST /{ig-user-id}/media with audio_configuration={audio_id,...}

use axum::{extract::{Path, Query, State}, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct MusicSearchQuery {
    pub q: Option<String>,
    /// "music" or "original_sound". Default: "music"
    pub audio_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MusicSearchResponse {
    pub tracks: Vec<MusicTrack>,
}

#[derive(Debug, Serialize)]
pub struct MusicTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub cover_url: Option<String>,
    pub duration_ms: Option<u64>,
    pub audio_type: String,
}

/// GET /api/integrations/:id/music?q=...
/// Search Instagram's audio library for music or original sounds.
/// If q is omitted, returns trending tracks.
pub async fn search_music(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<MusicSearchQuery>,
    auth: AuthenticatedUser,
) -> Result<Json<MusicSearchResponse>, AppError> {
    let integration = crate::db::queries::get_integration(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    // Only Instagram supports the Audio API
    if integration.provider_identifier != "instagram" && integration.provider_identifier != "instagram-standalone" {
        return Err(AppError::BadRequest(
            "Music search is only available for Instagram integrations".into(),
        ));
    }

    let token = state.token_key.as_ref()
        .and_then(|key| crate::crypto::decrypt_string(&integration.access_token, key).ok())
        .unwrap_or_else(|| integration.access_token.clone());

    let internal_id = &integration.internal_id;
    let audio_type = query.audio_type.as_deref().unwrap_or("music");

    // Build the Graph API URL
    let mut url = format!(
        "https://graph.facebook.com/v22.0/{}/ig_audio?audio_type={}&access_token={}",
        internal_id, audio_type, token
    );
    if let Some(ref q) = query.q {
        if !q.is_empty() {
            url.push_str(&format!("&search_query={}", urlencoding::encode(q)));
        }
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("IG Audio API request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::warn!("IG Audio API returned {status}: {body}");
        return Err(AppError::Internal(format!(
            "Instagram Audio API returned status {status}. The account may not support audio (requires Business/Creator account with Facebook Login flow)."
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse IG Audio response: {e}")))?;

    let tracks = json["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let id = item["id"].as_str()?.to_string();
                    let title = item["title"].as_str().unwrap_or("Unknown").to_string();
                    let artist = item["artist"]
                        .as_str()
                        .or_else(|| item["owner"]["username"].as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let cover_url = item["cover_artwork_uri"]
                        .as_str()
                        .or_else(|| item["display_image_uri"].as_str())
                        .map(String::from);
                    let duration_ms = item["duration_in_ms"].as_u64();
                    Some(MusicTrack {
                        id,
                        title,
                        artist,
                        cover_url,
                        duration_ms,
                        audio_type: audio_type.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(MusicSearchResponse { tracks }))
}
