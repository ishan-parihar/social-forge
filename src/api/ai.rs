// ─── AI API Routes ────────────────────────────────────────────
// Backend proxy for LLM calls. Uses LLM_ENDPOINT + LLM_MODEL from config.
// Keeps the API key server-side (was previously exposed in the frontend
// via VITE_PUBLIC_AI_PROXY_URL).

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct GeneratePostRequest {
    pub topic: String,
    pub tone: String,
    pub length: String,
}

#[derive(Debug, Deserialize)]
pub struct ContentRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeToneRequest {
    pub content: String,
    pub tone: String,
}

#[derive(Debug, Serialize)]
pub struct AiResponse {
    pub content: String,
}

async fn call_llm(
    state: &AppState,
    prompt: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<String, AppError> {
    let endpoint = state
        .config
        .llm_endpoint
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("LLM_ENDPOINT not configured".into()))?;
    let model = state
        .config
        .llm_model
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("LLM_MODEL not configured".into()))?;

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .json(&serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": temperature,
            "max_tokens": max_tokens,
        }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("LLM request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::Internal(format!(
            "LLM returned status {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse LLM response: {e}")))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AppError::Internal("LLM response missing content".into()))?
        .trim()
        .to_string();

    Ok(content)
}

/// POST /api/ai/generate-post
pub async fn generate_post(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<GeneratePostRequest>,
) -> Result<Json<AiResponse>, AppError> {
    let prompt = format!(
        "Write a {} social media post about \"{}\". Length: {}. Do not add hashtags.",
        body.tone, body.topic, body.length
    );
    let content = call_llm(&state, &prompt, 0.7, 500).await?;
    Ok(Json(AiResponse { content }))
}

/// POST /api/ai/improve-writing
pub async fn improve_writing(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<ContentRequest>,
) -> Result<Json<AiResponse>, AppError> {
    let prompt = format!(
        "Improve the following social media post for clarity and engagement. Keep the same message and length:\n\n{}",
        body.content
    );
    let content = call_llm(&state, &prompt, 0.5, 500).await?;
    Ok(Json(AiResponse { content }))
}

/// POST /api/ai/suggest-hashtags
pub async fn suggest_hashtags(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<ContentRequest>,
) -> Result<Json<AiResponse>, AppError> {
    let prompt = format!(
        "Extract 5-10 relevant hashtags from this content. Return ONLY the hashtags separated by spaces, no explanations:\n\n{}",
        body.content
    );
    let content = call_llm(&state, &prompt, 0.3, 100).await?;
    Ok(Json(AiResponse { content }))
}

/// POST /api/ai/change-tone
pub async fn change_tone(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<ChangeToneRequest>,
) -> Result<Json<AiResponse>, AppError> {
    let prompt = format!(
        "Rewrite the following post in a {} tone. Keep the same information and approximate length:\n\n{}",
        body.tone, body.content
    );
    let content = call_llm(&state, &prompt, 0.8, 500).await?;
    Ok(Json(AiResponse { content }))
}

/// POST /api/ai/summarize
pub async fn summarize(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<ContentRequest>,
) -> Result<Json<AiResponse>, AppError> {
    let prompt = format!(
        "Summarize the following content for an X/Twitter post. Maximum 280 characters. Return ONLY the post, no explanations:\n\n{}",
        body.content
    );
    let content = call_llm(&state, &prompt, 0.3, 150).await?;
    Ok(Json(AiResponse { content }))
}

// ── Phase 9: AI Generator (bulk post generation) ────────────────

#[derive(Debug, Deserialize)]
pub struct GenerateBulkRequest {
    pub topic: String,
    /// "one_short" | "one_long" | "thread_short" | "thread_long"
    pub format: String,
    /// "personal" | "company"
    pub tone: String,
    /// Number of posts to generate (default 3, max 5).
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_count() -> u32 {
    3
}

#[derive(Debug, Serialize)]
pub struct GenerateBulkResponse {
    pub posts: Vec<String>,
    /// Suggested schedule: one post per day starting tomorrow at 9am UTC.
    pub suggested_dates: Vec<String>,
}

/// POST /api/ai/generate-bulk
///
/// Generates multiple posts from a single topic prompt. Returns an array
/// of post contents + suggested schedule dates (one per day starting
/// tomorrow at 9am UTC).
///
/// The frontend GeneratorModal calls this, shows the results, and lets
/// the user pipe each one into the composer modal for review/scheduling.
pub async fn generate_bulk(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<GenerateBulkRequest>,
) -> Result<Json<GenerateBulkResponse>, AppError> {
    let count = body.count.clamp(1, 5);

    let format_desc = match body.format.as_str() {
        "one_short" => "a single short post (under 280 characters)",
        "one_long" => "a single long-form post (500-1000 characters)",
        "thread_short" => "a 3-tweet thread where each tweet is under 280 characters",
        "thread_long" => "a 5-tweet thread where each tweet is 200-400 characters",
        _ => "a single short post (under 280 characters)",
    };

    let tone_desc = match body.tone.as_str() {
        "personal" => "personal (first person, conversational, relatable)",
        "company" => "company (professional, brand voice, third person)",
        _ => "personal (first person, conversational, relatable)",
    };

    let prompt = format!(
        "Generate {count} distinct social media posts about \"{topic}\".\n\
         Format: {format_desc}\n\
         Tone: {tone_desc}\n\
         \n\
         Requirements:\n\
         - Each post must be unique — different angles, hooks, or aspects of the topic.\n\
         - Do NOT include hashtags unless they're naturally part of the content.\n\
         - Do NOT include numbering or labels like 'Post 1:' — just the content.\n\
         - Separate each post with a line containing only '---'.\n\
         \n\
         Return ONLY the posts separated by '---', no explanations.",
        count = count,
        topic = body.topic,
        format_desc = format_desc,
        tone_desc = tone_desc,
    );

    let raw = call_llm(&state, &prompt, 0.8, 2000).await?;

    // Split the response by '---' and trim each piece.
    let posts: Vec<String> = raw
        .split("---")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(count as usize)
        .collect();

    if posts.is_empty() {
        return Err(AppError::Internal(
            "AI returned no valid posts. Please try again.".into(),
        ));
    }

    // Suggested schedule: one post per day starting tomorrow at 9am UTC.
    let now = chrono::Utc::now();
    let suggested_dates: Vec<String> = (0..posts.len() as i64)
        .map(|i| {
            let d = now + chrono::Duration::days(i + 1);
            d.format("%Y-%m-%dT09:00:00Z").to_string()
        })
        .collect();

    Ok(Json(GenerateBulkResponse { posts, suggested_dates }))
}
