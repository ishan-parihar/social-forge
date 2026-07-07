// ─── Brand Profile API Routes ──────────────────────────────────
// v24-4: CRUD for the brand profile (brand name, tone of voice,
// audience, content pillars, keywords, hashtag sets, avoid topics,
// posting frequency, posts_per_day_goal).
//
// The brand profile is used by:
//   - The AiAssistant as context for generate/improve/tone (so AI-
//     generated content matches the brand's voice and audience).
//   - The analytics cadence endpoint for goal_per_day (so the dashboard
//     can show actual vs goal posts-per-day).
//
// Previously the brand profile was stored in localStorage only — not
// synced across devices and not read by the AiAssistant. This module
// persists it to the brand_profiles table (migration 036).

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BrandProfile {
    pub user_id: Uuid,
    pub brand_name: Option<String>,
    pub description: Option<String>,
    pub tone_of_voice: Option<String>,
    pub audience: Option<String>,
    pub content_pillars: Option<serde_json::Value>,
    pub keywords: Option<serde_json::Value>,
    pub hashtag_sets: Option<serde_json::Value>,
    pub avoid_topics: Option<serde_json::Value>,
    pub posting_frequency: Option<String>,
    pub posts_per_day_goal: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateBrandProfileRequest {
    pub brand_name: Option<String>,
    pub description: Option<String>,
    pub tone_of_voice: Option<String>,
    pub audience: Option<String>,
    pub content_pillars: Option<serde_json::Value>,
    pub keywords: Option<serde_json::Value>,
    pub hashtag_sets: Option<serde_json::Value>,
    pub avoid_topics: Option<serde_json::Value>,
    pub posting_frequency: Option<String>,
    pub posts_per_day_goal: Option<f64>,
}

/// GET /api/profile — get the current user's brand profile.
/// Returns 200 with an empty body if no profile exists yet.
pub async fn get_profile(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Option<BrandProfile>>, AppError> {
    let profile: Option<BrandProfile> = sqlx::query_as(
        r#"SELECT user_id, brand_name, description, tone_of_voice, audience,
                  content_pillars, keywords, hashtag_sets, avoid_topics,
                  posting_frequency, posts_per_day_goal, created_at, updated_at
           FROM brand_profiles WHERE user_id = $1"#,
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;
    Ok(Json(profile))
}

/// PUT /api/profile — upsert the brand profile.
/// Creates a new row if none exists; updates the existing row otherwise.
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<UpdateBrandProfileRequest>,
) -> Result<Json<BrandProfile>, AppError> {
    let profile: BrandProfile = sqlx::query_as(
        r#"INSERT INTO brand_profiles (
               user_id, brand_name, description, tone_of_voice, audience,
               content_pillars, keywords, hashtag_sets, avoid_topics,
               posting_frequency, posts_per_day_goal
           ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           ON CONFLICT (user_id) DO UPDATE SET
               brand_name = COALESCE(EXCLUDED.brand_name, brand_profiles.brand_name),
               description = COALESCE(EXCLUDED.description, brand_profiles.description),
               tone_of_voice = COALESCE(EXCLUDED.tone_of_voice, brand_profiles.tone_of_voice),
               audience = COALESCE(EXCLUDED.audience, brand_profiles.audience),
               content_pillars = COALESCE(EXCLUDED.content_pillars, brand_profiles.content_pillars),
               keywords = COALESCE(EXCLUDED.keywords, brand_profiles.keywords),
               hashtag_sets = COALESCE(EXCLUDED.hashtag_sets, brand_profiles.hashtag_sets),
               avoid_topics = COALESCE(EXCLUDED.avoid_topics, brand_profiles.avoid_topics),
               posting_frequency = COALESCE(EXCLUDED.posting_frequency, brand_profiles.posting_frequency),
               posts_per_day_goal = COALESCE(EXCLUDED.posts_per_day_goal, brand_profiles.posts_per_day_goal),
               updated_at = NOW()
           RETURNING user_id, brand_name, description, tone_of_voice, audience,
                     content_pillars, keywords, hashtag_sets, avoid_topics,
                     posting_frequency, posts_per_day_goal, created_at, updated_at"#,
    )
    .bind(auth.user_id)
    .bind(&body.brand_name)
    .bind(&body.description)
    .bind(&body.tone_of_voice)
    .bind(&body.audience)
    .bind(&body.content_pillars)
    .bind(&body.keywords)
    .bind(&body.hashtag_sets)
    .bind(&body.avoid_topics)
    .bind(&body.posting_frequency)
    .bind(body.posts_per_day_goal)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(profile))
}
