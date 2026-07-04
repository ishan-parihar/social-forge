// ─── LinkedIn Provider ────────────────────────────────────────
// OAuth 2.0 PKCE. Posts to LinkedIn profile or page.
// Uses LinkedIn API v2 (https://api.linkedin.com/v2).

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct LinkedInProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl LinkedInProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("linkedin")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn token_url(&self) -> &'static str {
        "https://www.linkedin.com/oauth/v2/accessToken"
    }

    fn authorize_url(&self) -> &'static str {
        "https://www.linkedin.com/oauth/v2/authorization"
    }

    // ─── API Methods ──────────────────────────────────────────

    pub async fn get_profile(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let url = "https://api.linkedin.com/v2/userinfo";
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_user_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let profile = self.get_profile(access_token).await?;
        let user_id = profile["sub"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Could not get LinkedIn user ID".into()))?
            .to_string();
        Ok(user_id)
    }

    pub async fn get_posts(
        &self,
        access_token: &str,
        author_urn: &str,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.clamp(1, 100);
        // URL-encode the author URN (contains colons like "urn:li:person:abc123")
        let encoded_author = urlencoding::encode(author_urn);
        let url = format!(
            "https://api.linkedin.com/v2/rest/posts?author={encoded_author}&count={limit}"
        );
        tracing::debug!("LinkedIn get_posts: calling {}", url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            tracing::debug!(
                "LinkedIn get_posts SUCCESS — response keys: {:?}, elements count: {}",
                json.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
                json["elements"].as_array().map(|a| a.len()).unwrap_or(0),
            );
            Ok(json)
        } else if status == 401 {
            // Don't log the full response body — LinkedIn error responses
            // can echo the requested Bearer token or URN. Log status only.
            tracing::warn!("LinkedIn get_posts 401 — token expired");
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .or_else(|| json["error_description"].as_str())
                .unwrap_or("LinkedIn API error")
                .to_string();
            tracing::warn!(
                "LinkedIn get_posts failed with status {}: {}. Full response: {:#}",
                status, msg, json
            );
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_post_detail(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.linkedin.com/v2/rest/posts/{post_urn}");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_post_comments_linkedin(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/comments"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn delete_post(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<(), ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/posts/{}",
            urlencoding::encode(post_urn)
        );
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() || status.as_u16() == 204 {
            Ok(())
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ProviderError::Api(format!("Delete failed ({}): {}", status, body)))
        }
    }

    pub async fn get_reactions(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/likes"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("LinkedIn API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_shares(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/shares"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("LinkedIn API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn create_comment(
        &self,
        access_token: &str,
        post_urn: &str,
        actor_urn: &str,
        message: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/comments"
        );
        let body = serde_json::json!({
            "actor": actor_urn,
            "message": {
                "text": message,
            },
            "object": post_urn,
        });
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status == 201 {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Resolve a LinkedIn media URN (e.g. "urn:li:image:12345") to a direct download/playback URL.
    /// Uses the LinkedIn Images or Videos REST API to get the actual CDN URL.
    /// Returns None if resolution fails (e.g. expired token, missing permission).
    pub async fn resolve_media_url(&self, access_token: &str, media_urn: &str) -> Option<String> {
        // Extract asset type and ID from URN: "urn:li:image:ABC" -> ("image", "ABC")
        let parts: Vec<&str> = media_urn.split(':').collect();
        if parts.len() < 4 {
            tracing::debug!("LinkedIn: invalid media URN format: {media_urn}");
            return None;
        }
        let asset_type = parts[2]; // "image" or "video"
        let asset_id = parts[3];

        let base_url = match asset_type {
            "image" => "https://api.linkedin.com/rest/images",
            "video" => "https://api.linkedin.com/rest/videos",
            _ => {
                tracing::debug!("LinkedIn: unsupported media type '{asset_type}' in URN {media_urn}");
                return None;
            }
        };

        // Primary: GET the asset directly — this returns metadata including download/playback URLs
        let url = format!("{base_url}/{asset_id}");
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.http
                .get(&url)
                .header("Authorization", format!("Bearer {access_token}"))
                .header("X-Restli-Protocol-Version", "2.0.0")
                .header("LinkedIn-Version", "202401")
                .send(),
        ).await;

        match resp {
            Ok(Ok(r)) if r.status().is_success() => {
                if let Ok(json) = r.json::<serde_json::Value>().await {
                    // Direct asset GET may return {"value": {"downloadUrl": "..."}}
                    if let Some(dl) = json.pointer("/value/downloadUrl").and_then(|v| v.as_str()) {
                        return Some(dl.to_string());
                    }
                    // Or {"value": {"playbackStreams": [...]}}
                    if let Some(streams) = json.pointer("/value/playbackStreams").and_then(|v| v.as_array()) {
                        if let Some(stream) = streams.first() {
                            if let Some(s) = stream.get("streamLocations").and_then(|v| v.as_array()) {
                                if let Some(loc) = s.first() {
                                    if let Some(u) = loc.get("url").and_then(|v| v.as_str()) {
                                        return Some(u.to_string());
                                    }
                                }
                            }
                        }
                    }
                    // Fallback: recursively search for any URL in the response
                    if let Some(url_val) = find_first_url(&json) {
                        return Some(url_val);
                    }
                }
            }
            Ok(Ok(r)) => {
                tracing::debug!("LinkedIn: resolve_media_url HTTP {} for {media_urn}", r.status());
            }
            Ok(Err(e)) => {
                tracing::debug!("LinkedIn: resolve_media_url network error for {media_urn}: {e}");
            }
            Err(_) => {
                tracing::debug!("LinkedIn: resolve_media_url timed out for {media_urn}");
            }
        }

        None
    }
}

#[async_trait]
impl SocialProvider for LinkedInProvider {
    fn identifier(&self) -> &'static str {
        "linkedin"
    }

    fn name(&self) -> &'static str {
        "LinkedIn"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "openid".into(),
            "profile".into(),
            "email".into(),
            "w_member_social".into(),
            // NOTE: r_member_social is needed for feed import (GET /v2/rest/posts)
            // but requires the "Read member's social" product enabled on the LinkedIn
            // app. Without it, the OAuth flow shows "Bummer, something went wrong".
            // We include it here; if the app doesn't have it, the feed refresher
            // marks the integration as refresh_needed so the user can reconnect.
        ]
    }

    fn max_content_length(&self) -> usize {
        3000
    }

    fn validate_media(&self, post: &PostContent) -> Result<(), String> {
        super::validate_media_limits(self.identifier(), post)
    }

    /// LinkedIn uses rotating refresh tokens that expire after ~60 days.
    /// Proactive refresh ensures tokens don't silently expire between API calls.
    fn needs_cron_refresh(&self) -> bool {
        true
    }

    /// LinkedIn needs a 10-second propagation delay after token rotation.
    fn refresh_wait(&self) -> bool {
        true
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", redirect_uri),
            ("scope", &self.scopes().join(" ")),
            ("state", state),
        ];

        let url = url::Url::parse_with_params(self.authorize_url(), &params)
            .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;

        Ok(AuthUrlResponse { url: url.to_string() })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // LinkedIn uses client_secret_post, not PKCE
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let resp = self.http.post(self.token_url()).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        // Get user profile
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let sub = profile["sub"].as_str().unwrap_or("").to_string();
        let name = profile["name"].as_str().unwrap_or("").to_string();
        let username = profile["preferred_username"].as_str().unwrap_or("").to_string();
        let picture = profile["picture"].as_str().map(String::from);

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: sub,
            name,
            username,
            picture,
        })
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let resp = self.http.post(self.token_url()).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Fetch profile info so it's not lost on refresh
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(AuthToken {
            access_token,
            refresh_token: json["refresh_token"].as_str().map(String::from),
            expires_in: json["expires_in"].as_u64().map(|v| v as u32),
            provider_user_id: profile["sub"].as_str().unwrap_or("").to_string(),
            name: profile["name"].as_str().unwrap_or("").to_string(),
            username: profile["preferred_username"].as_str().unwrap_or("").to_string(),
            picture: profile["picture"].as_str().map(String::from),
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Get user profile ID for posting
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let user_id = profile["sub"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Could not get user profile".into()))?;

        let body = serde_json::json!({
            "author": format!("urn:li:person:{user_id}"),
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": post.content,
                    },
                    "shareMediaCategory": if post.media.is_empty() { "NONE" } else { "IMAGE" },
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC"
            }
        });

        let resp = self
            .http
            .post("https://api.linkedin.com/v2/ugcPosts")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == 201 {
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let post_id = location.rsplit('/').next().unwrap_or("").to_string();
            return Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: None, // LinkedIn doesn't always provide URL
                status: "published".into(),
            });
        }

        let json: serde_json::Value = resp.json().await?;

        if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .or_else(|| json["error_description"].as_str())
                .unwrap_or("LinkedIn publish failed")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        _page_id: &str,
    ) -> Result<super::ReconnectResult, ProviderError> {
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(super::ReconnectResult {
            id: profile["sub"].as_str().unwrap_or("").to_string(),
            name: profile["name"].as_str().unwrap_or("").to_string(),
            access_token: access_token.to_string(),
            picture: profile["picture"].as_str().map(String::from),
            username: profile["preferred_username"].as_str().map(String::from),
        })
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        // LinkedIn personal profiles: get network size (connections/followers)
        let user_id = self.get_user_id(access_token).await?;
        let person_urn = format!("urn:li:person:{user_id}");

        let mut results = Vec::new();

        // Get follower/connection count
        let url = format!(
            "https://api.linkedin.com/v2/networkSizes/{}?edgeType=CompanyFollowedByMember",
            person_urn
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(count) = json["firstDegreeSize"].as_u64() {
                results.push(AnalyticsData {
                    label: "connections".into(),
                    data: vec![AnalyticsDataPoint {
                        total: count.to_string(),
                        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    }],
                    percentage_change: 0.0,
                });
            }
        }

        Ok(results)
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        // Get social actions (likes, comments, shares counts) for a post
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{platform_post_id}"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if !status.is_success() {
            let msg = json["message"].as_str().unwrap_or("LinkedIn post analytics error").to_string();
            return Err(ProviderError::Api(msg));
        }

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut results = Vec::new();

        if let Some(n) = json["likesSummary"]["totalLikes"].as_u64() {
            results.push(AnalyticsData {
                label: "likes".into(),
                data: vec![AnalyticsDataPoint { total: n.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(n) = json["commentsSummary"]["totalFirstLevelComments"].as_u64() {
            results.push(AnalyticsData {
                label: "comments".into(),
                data: vec![AnalyticsDataPoint { total: n.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(n) = json["sharesSummary"]["totalShares"].as_u64() {
            results.push(AnalyticsData {
                label: "shares".into(),
                data: vec![AnalyticsDataPoint { total: n.to_string(), date: today }],
                percentage_change: 0.0,
            });
        }

        Ok(results)
    }

    async fn get_recent_posts(
        &self,
        access_token: &str,
        _internal_id: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        let user_id = self.get_user_id(access_token).await?;
        let author_urn = format!("urn:li:person:{user_id}");
        let json = self.get_posts(access_token, &author_urn, limit).await?;

        // Fetch profile info for author details (name, handle, avatar)
        let profile = self.get_profile(access_token).await.ok();
        let author_name = profile.as_ref().and_then(|p| p["name"].as_str().map(String::from));
        let author_handle = profile.as_ref().and_then(|p| p["preferred_username"].as_str().map(String::from));
        let author_avatar = profile.as_ref().and_then(|p| p["picture"].as_str().map(String::from));

        tracing::info!(
            "LinkedIn get_recent_posts: {} elements in response, user_id={}",
            json["elements"].as_array().map(|a| a.len()).unwrap_or(0),
            user_id,
        );

        let mut posts = Vec::new();
        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                let post_urn = element["id"].as_str().unwrap_or("").to_string();
                if post_urn.is_empty() {
                    tracing::warn!("LinkedIn get_recent_posts: element has no id: {:#}", element);
                    continue;
                }

                // LinkedIn Posts API (202601) may return `commentary` as:
                // - A string: "My post content"
                // - An object: { "text": "My post content" }
                // Handle both formats gracefully.
                let commentary = element["commentary"]
                    .as_str()
                    .or_else(|| {
                        element["commentary"]
                            .as_object()
                            .and_then(|c| c["text"].as_str())
                    })
                    .unwrap_or("")
                    .to_string();

                let created_at = element["createdAt"].as_i64()
                    .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms))
                    .unwrap_or_else(chrono::Utc::now);

                tracing::debug!(
                    "LinkedIn post parsed: id={} text_len={} created={:?}",
                    post_urn,
                    commentary.len(),
                    created_at,
                );

                // Extract media from the post content
                let mut media = Vec::new();
                // Limit API calls: at most 4 media items per post to avoid N+1 latency
                const MAX_MEDIA_PER_POST: usize = 4;

                if let Some(content) = element.get("content") {
                    if !content.is_null() {
                        // Single media: content.media { id: "urn:li:image:xxx" }
                        if let Some(m) = content.get("media") {
                            if let Some(media_urn) = m.get("id").and_then(|v| v.as_str()) {
                                if let Some(url) = self.resolve_media_url(access_token, media_urn).await {
                                    let is_video = media_urn.contains(":video:");
                                    media.push(MediaAttachment {
                                        url,
                                        mime_type: if is_video { "video/mp4".to_string() } else { "image/jpeg".to_string() },
                                        alt: m.get("title").and_then(|v| v.as_str()).map(String::from),
                                        poster_url: None,
                                    });
                                }
                            }
                        }

                        // Multi-image / carousel: content.multiImage array
                        if media.is_empty() {
                            if let Some(multi) = content.get("multiImage").and_then(|v| v.as_array()) {
                                for img in multi.iter().take(MAX_MEDIA_PER_POST) {
                                    if let Some(media_urn) = img.get("id").and_then(|v| v.as_str()) {
                                        if let Some(url) = self.resolve_media_url(access_token, media_urn).await {
                                            media.push(MediaAttachment {
                                                url,
                                                mime_type: "image/jpeg".to_string(),
                                                alt: None,
                                                poster_url: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        // Article with thumbnail
                        if media.is_empty() {
                            if let Some(article) = content.get("article") {
                                if let Some(thumb) = article.get("thumbnail").and_then(|v| v.as_str()) {
                                    if !thumb.is_empty() {
                                        media.push(MediaAttachment {
                                            url: thumb.to_string(),
                                            mime_type: "image/jpeg".to_string(),
                                            alt: article.get("title").and_then(|v| v.as_str()).map(String::from),
                                            poster_url: None,
                                        });
                                    }
                                }
                            }
                        }

                        // Single file: content.singlefile (document/PDF)
                        if media.is_empty() {
                            if let Some(file) = content.get("singlefile") {
                                if let Some(media_urn) = file.get("id").and_then(|v| v.as_str()) {
                                    if let Some(url) = self.resolve_media_url(access_token, media_urn).await {
                                        media.push(MediaAttachment {
                                            url,
                                            mime_type: "application/pdf".to_string(),
                                            alt: file.get("title").and_then(|v| v.as_str()).map(String::from),
                                            poster_url: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // Build LinkedIn post URL
                let post_url = Some(format!("https://www.linkedin.com/feed/update/{post_urn}"));

                posts.push(ExternalPostData {
                    platform_post_id: post_urn,
                    text: commentary,
                    author_name: author_name.clone(),
                    author_handle: author_handle.clone(),
                    author_avatar: author_avatar.clone(),
                    created_at,
                    url: post_url,
                    media,
                    metadata: Some(element.clone()),
                });
            }
        } else if json["elements"].is_null() {
            tracing::warn!(
                "LinkedIn get_recent_posts: 'elements' is null/absent. Response keys: {:?}",
                json.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
            );
        }
        Ok(posts)
    }

    async fn get_post_engagement(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        // Use the social actions API to get likes/comments/shares counts
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{platform_post_id}"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if !status.is_success() {
            return Ok(None);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let mut result = serde_json::Map::new();

        if let Some(n) = json["likesSummary"]["totalLikes"].as_i64() {
            result.insert("likeCount".into(), serde_json::json!(n));
        }
        if let Some(n) = json["commentsSummary"]["totalFirstLevelComments"].as_i64() {
            result.insert("commentCount".into(), serde_json::json!(n));
        }
        if let Some(n) = json["sharesSummary"]["totalShares"].as_i64() {
            result.insert("shareCount".into(), serde_json::json!(n));
        }

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(serde_json::Value::Object(result)))
        }
    }

    async fn get_post_comments(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<CommentData>, ProviderError> {
        let json = self.get_post_comments_linkedin(access_token, platform_post_id).await?;

        let mut comments = Vec::new();
        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                let id = element["id"].as_str().unwrap_or("").to_string();
                let text = element["message"]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let created_at = element["createdAt"].as_i64()
                    .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms))
                    .unwrap_or_else(chrono::Utc::now);

                let author = element["actor"].as_str().map(|a| {
                    // Extract name from URN, e.g., "urn:li:person:{id}"
                    a.rsplit(':').next().unwrap_or(a).to_string()
                });

                let like_count = element["likesSummary"]["totalLikes"]
                    .as_i64()
                    .unwrap_or(0) as i32;

                // LinkedIn comments API v2 doesn't include nested replies in the same endpoint
                let replies = Vec::new();

                comments.push(CommentData {
                    id,
                    author_name: author.clone(),
                    author_avatar: None,
                    text,
                    created_at,
                    like_count,
                    replies,
                });
            }
        }
        Ok(comments)
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api("LinkedIn personal profile does not support page selection".into()))
    }
}

impl LinkedInProvider {
    pub async fn reply_to_comment(
        &self,
        access_token: &str,
        comment_id: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let user_id = profile["sub"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Could not get user profile".into()))?;

        let body = serde_json::json!({
            "author": format!("urn:li:person:{user_id}"),
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": post.content,
                    },
                    "shareMediaCategory": "NONE",
                    "parentComment": comment_id,
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC"
            }
        });

        let resp = self
            .http
            .post("https://api.linkedin.com/v2/ugcPosts")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == 201 {
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let post_id = location.rsplit('/').next().unwrap_or("").to_string();
            return Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: None,
                status: "published".into(),
            });
        }

        let json: serde_json::Value = resp.json().await?;
        let msg = json["message"]
            .as_str()
            .or_else(|| json["error_description"].as_str())
            .unwrap_or("LinkedIn reply failed")
            .to_string();
        Err(ProviderError::Api(msg))
    }

    pub async fn send_dm(
        &self,
        access_token: &str,
        recipient: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let body = serde_json::json!({
            "recipients": {
                "elements": [{
                    "id": recipient,
                }]
            },
            "messageBody": {
                "text": post.content,
            },
        });

        let resp = self
            .http
            .post("https://api.linkedin.com/v2/messages")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == 201 {
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let message_id = location.rsplit('/').next().unwrap_or("").to_string();
            return Ok(PublishResult {
                platform_post_id: message_id,
                platform_post_url: None,
                status: "sent".into(),
            });
        }

        let json: serde_json::Value = resp.json().await?;
        let msg = json["message"]
            .as_str()
            .or_else(|| json["error_description"].as_str())
            .unwrap_or("LinkedIn send DM failed")
            .to_string();
        Err(ProviderError::Api(msg))
    }

    pub async fn get_dm_conversations(
        &self,
        access_token: &str,
        limit: u32,
    ) -> Result<Vec<super::DmConversation>, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/messages?action=search&count={}",
            limit.min(50)
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let mut conversations = Vec::new();

        if let Some(elements) = json["elements"].as_array() {
            for elem in elements {
                let id = elem["conversationId"].as_str().unwrap_or("").to_string();
                let participants = elem["participants"]
                    .as_array()
                    .and_then(|p| p.first())
                    .and_then(|p| p["id"].as_str())
                    .unwrap_or("")
                    .to_string();
                let last_message = elem["body"].as_str().map(|s| s.to_string());
                let last_message_at = elem["createdAt"]
                    .as_i64()
                    .map(|ts| chrono::DateTime::from_timestamp(ts / 1000, 0))
                    .flatten();

                conversations.push(super::DmConversation {
                    id,
                    participant: participants,
                    participant_name: None,
                    participant_avatar: None,
                    last_message,
                    last_message_at,
                    unread_count: 0,
                });
            }
        }

        Ok(conversations)
    }

    pub async fn get_dm_messages(
        &self,
        access_token: &str,
        conversation_id: &str,
        limit: u32,
    ) -> Result<Vec<super::DmMessage>, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/messages?action=byConversation&conversationId={}&count={}",
            conversation_id,
            limit.min(50)
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202401")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let mut messages = Vec::new();

        if let Some(elements) = json["elements"].as_array() {
            for elem in elements {
                let id = elem["id"].as_str().unwrap_or("").to_string();
                let sender = elem["sender"]
                    .as_object()
                    .and_then(|s| s.get("id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or("")
                    .to_string();
                let content = elem["body"].as_str().unwrap_or("").to_string();
                let created_at = elem["createdAt"]
                    .as_i64()
                    .map(|ts| chrono::DateTime::from_timestamp(ts / 1000, 0))
                    .flatten()
                    .unwrap_or_else(chrono::Utc::now);

                messages.push(super::DmMessage {
                    id,
                    conversation_id: conversation_id.to_string(),
                    sender,
                    sender_name: None,
                    content,
                    media: vec![],
                    created_at,
                    read: true,
                });
            }
        }

        Ok(messages)
    }
}

/// Recursively search a JSON value for the first string that looks like a URL.
fn find_first_url(val: &serde_json::Value) -> Option<String> {
    match val {
        serde_json::Value::String(s) => {
            if s.starts_with("http://") || s.starts_with("https://") {
                Some(s.clone())
            } else {
                None
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(url) = find_first_url(item) {
                    return Some(url);
                }
            }
            None
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map {
                if let Some(url) = find_first_url(v) {
                    return Some(url);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            database_url: "sqlite:test".into(),
            jwt_secret: "test".into(),
            app_password: "test".into(),
            app_url: "http://localhost:3000".into(),
            frontend_url: "http://localhost:4200".into(),
            x_client_id: None,
            x_client_secret: None,
            x_auth_token: None,
            x_ct0: None,
            linkedin_client_id: Some("test_linkedin_id".into()),
            linkedin_client_secret: Some("test_linkedin_secret".into()),
            bluesky_handle: None,
            bluesky_app_password: None,
            facebook_client_id: None,
            facebook_client_secret: None,
            instagram_client_id: None,
            instagram_client_secret: None,
            threads_app_id: None,
            threads_app_secret: None,
            youtube_client_id: None,
            youtube_client_secret: None,
            reddit_client_id: None,
            reddit_client_secret: None,
            reddit_username: None,
            reddit_password: None,
            reddit_access_token: None,
            reddit_refresh_token: None,
            discord_client_id: None,
            discord_client_secret: None,
            discord_bot_token: None,
            telegram_bot_tokens: None,
            telegram_session_dir: None,
            telegram_api_id: None,
            telegram_api_hash: None,
            tiktok_client_id: None,
            tiktok_client_secret: None,
            medium_access_token: None,
            devto_api_key: None,
            pinterest_client_id: None,
            pinterest_client_secret: None,
            whatsapp_store_dir: None,
            slack_client_id: None,
            slack_client_secret: None,
            instagram_app_id: None,
            instagram_app_secret: None,
            mastodon_client_id: None,
            mastodon_client_secret: None,
            mastodon_instance_url: None,
            hashnode_api_key: None,
            github_token: None,
            vk_client_id: None,
            vk_client_secret: None,
            whop_client_id: None,
            whop_client_secret: None,
            kick_client_id: None,
            kick_client_secret: None,
            neynar_api_key: None,
            token_encryption_key: None,
            media_dir: "./uploads".into(),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            stripe_price_free: None,
            stripe_price_pro_monthly: None,
            stripe_price_pro_annual: None,
            stripe_price_business_monthly: None,
            stripe_price_business_annual: None,
            llm_endpoint: None,
            llm_model: None,
        }
    }

    #[test]
    fn test_scopes_contain_required() {
        let provider = LinkedInProvider::new(&test_config());
        let scopes = provider.scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"w_member_social".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
    }

    #[test]
    fn test_identifier_and_name() {
        let provider = LinkedInProvider::new(&test_config());
        assert_eq!(provider.identifier(), "linkedin");
        assert_eq!(provider.name(), "LinkedIn");
    }

    #[test]
    fn test_max_content_length() {
        let provider = LinkedInProvider::new(&test_config());
        assert_eq!(provider.max_content_length(), 3000);
    }

    #[tokio::test]
    async fn test_generate_auth_url_contains_params() {
        let provider = LinkedInProvider::new(&test_config());
        let result = provider.generate_auth_url("test_state", "test_verifier", "http://localhost:3000/callback").await;
        let url = result.unwrap().url;

        assert!(url.contains("response_type=code"), "should contain response_type=code");
        assert!(url.contains("client_id=test_linkedin_id"), "should contain client_id");
        assert!(url.contains("redirect_uri="), "should contain redirect_uri");
        assert!(url.contains("state=test_state"), "should contain state");
        assert!(url.contains("scope="), "should contain scope");
        assert!(url.starts_with("https://www.linkedin.com/oauth/v2/authorization"));
    }
}
