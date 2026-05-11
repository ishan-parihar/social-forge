// ─── X/Twitter Provider ───────────────────────────────────────
// Uses OAuth 2.0 PKCE via Twitter API v2.

use async_trait::async_trait;

use super::*;
use crate::config::Config;
use chrono::Utc;

pub struct XProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl XProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("x")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn oauth_token_endpoint(&self) -> &'static str {
        "https://api.twitter.com/2/oauth2/token"
    }

    fn oauth_authorize_endpoint(&self) -> &'static str {
        "https://twitter.com/i/oauth2/authorize"
    }
}

#[async_trait]
impl SocialProvider for XProvider {
    fn identifier(&self) -> &'static str {
        "x"
    }

    fn name(&self) -> &'static str {
        "X (Twitter)"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "tweet.read".into(),
            "tweet.write".into(),
            "users.read".into(),
            "offline.access".into(),
            "bookmark.read".into(),
            "bookmark.write".into(),
            "like.read".into(),
            "like.write".into(),
            "follows.read".into(),
            "follows.write".into(),
            "list.read".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        4000
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let challenge = common::generate_code_challenge(code_verifier);

        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", redirect_uri),
            ("scope", &self.scopes().join(" ")),
            ("state", state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ];

        let url = url::Url::parse_with_params(self.oauth_authorize_endpoint(), &params)
            .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;

        Ok(AuthUrlResponse {
            url: url.to_string(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // X/Twitter OAuth 2.0 PKCE — public client, no client_secret needed
        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );
        let json: serde_json::Value = self
            .http
            .post(self.oauth_token_endpoint())
            .header("Authorization", format!("Basic {credentials}"))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("code_verifier", code_verifier),
                ("redirect_uri", redirect_uri),
                ("client_id", &self.client_id),
            ])
            .send()
            .await?
            .json()
            .await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token in response".into()))?
            .to_string();

        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        // Get user info
        let user_info = self
            .http
            .get("https://api.twitter.com/2/users/me")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let provider_user_id = user_info["data"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let name = user_info["data"]["name"].as_str().unwrap_or("").to_string();
        let username = user_info["data"]["username"].as_str().unwrap_or("").to_string();

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id,
            name,
            username,
            picture: None,
        })
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );
        let json: serde_json::Value = self
            .http
            .post(self.oauth_token_endpoint())
            .header("Authorization", format!("Basic {credentials}"))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", &self.client_id),
            ])
            .send()
            .await?
            .json()
            .await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        let new_refresh = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        Ok(AuthToken {
            access_token,
            refresh_token: new_refresh,
            expires_in,
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let mut body = serde_json::json!({
            "text": post.content,
        });

        // Add media if present
        if !post.media.is_empty() {
            let media_ids = self.upload_media(access_token, &post.media).await?;
            if !media_ids.is_empty() {
                body["media"] = serde_json::json!({ "media_ids": media_ids });
            }
        }

        let resp = self
            .http
            .post("https://api.twitter.com/2/tweets")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 201 || status == 200 {
            let post_id = json["data"]["id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            Ok(PublishResult {
                platform_post_url: Some(format!("https://twitter.com/user/status/{post_id}")),
                platform_post_id: post_id.clone(),
                status: "published".into(),
            })
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json["title"].as_str() == Some("Unauthorized")
            || json["status"].as_i64() == Some(401)
        {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["detail"].as_str().unwrap_or("Unknown error").to_string(),
            ))
        }
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api("X does not support page management".into()))
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let resp = self
            .http
            .get("https://api.twitter.com/2/users/me?user.fields=public_metrics")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            let metrics = &json["data"]["public_metrics"];
            let mut result = Vec::new();
            if let Some(followers) = metrics["followers_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Followers".into(),
                    data: vec![AnalyticsDataPoint {
                        total: followers.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(following) = metrics["following_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Following".into(),
                    data: vec![AnalyticsDataPoint {
                        total: following.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(tweets) = metrics["tweet_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Tweets".into(),
                    data: vec![AnalyticsDataPoint {
                        total: tweets.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(listed) = metrics["listed_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Listed".into(),
                    data: vec![AnalyticsDataPoint {
                        total: listed.to_string(),
                        date: today,
                    }],
                    percentage_change: 0.0,
                });
            }
            Ok(result)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let resp = self
            .http
            .get(format!("https://api.twitter.com/2/tweets/{platform_post_id}?tweet.fields=public_metrics"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            let metrics = &json["data"]["public_metrics"];
            let mut result = Vec::new();
            if let Some(likes) = metrics["like_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Likes".into(),
                    data: vec![AnalyticsDataPoint {
                        total: likes.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(retweets) = metrics["retweet_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Retweets".into(),
                    data: vec![AnalyticsDataPoint {
                        total: retweets.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(replies) = metrics["reply_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Replies".into(),
                    data: vec![AnalyticsDataPoint {
                        total: replies.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(quotes) = metrics["quote_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Quotes".into(),
                    data: vec![AnalyticsDataPoint {
                        total: quotes.to_string(),
                        date: today.clone(),
                    }],
                    percentage_change: 0.0,
                });
            }
            if let Some(impressions) = metrics["impression_count"].as_i64() {
                result.push(AnalyticsData {
                    label: "Impressions".into(),
                    data: vec![AnalyticsDataPoint {
                        total: impressions.to_string(),
                        date: today,
                    }],
                    percentage_change: 0.0,
                });
            }
            Ok(result)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }
}

impl XProvider {
    /// Download media from URL or read from local filesystem
    async fn fetch_media_bytes(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        if url.starts_with("http://") || url.starts_with("https://") {
            let resp = self.http.get(url).send().await.map_err(|e| {
                ProviderError::Api(format!("Failed to fetch media from {url}: {e}"))
            })?;
            let status = resp.status();
            if !status.is_success() {
                return Err(ProviderError::Api(format!(
                    "Failed to fetch media: HTTP {status}"
                )));
            }
            resp.bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| ProviderError::Api(format!("Failed to read media body: {e}")))
        } else {
            // Local filesystem path
            tokio::fs::read(url)
                .await
                .map_err(|e| ProviderError::Api(format!("Failed to read local media {url}: {e}")))
        }
    }

    /// Upload media to Twitter using v1.1 media/upload (INIT → APPEND → FINALIZE)
    async fn upload_single_media(
        &self,
        access_token: &str,
        media_url: &str,
        mime_type: &str,
    ) -> Result<String, ProviderError> {
        let bytes = self.fetch_media_bytes(media_url).await?;
        let total_bytes = bytes.len();

        // Determine media category for Twitter
        let media_category = if mime_type.starts_with("video/") {
            "tweet_video"
        } else if mime_type == "image/gif" {
            "tweet_gif"
        } else {
            "tweet_image"
        };

        let api_base = "https://upload.twitter.com/1.1/media/upload.json";

        // ── INIT ────────────────────────────────────────────
        let init_resp: serde_json::Value = self
            .http
            .post(api_base)
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&[
                ("command", "INIT"),
                ("total_bytes", &total_bytes.to_string()),
                ("media_type", mime_type),
                ("media_category", media_category),
            ])
            .send()
            .await?
            .json()
            .await?;

        let media_id = init_resp["media_id_string"]
            .as_str()
            .ok_or_else(|| {
                ProviderError::Api(format!(
                    "Twitter INIT failed: {:?}",
                    init_resp
                ))
            })?
            .to_string();

        // ── APPEND ──────────────────────────────────────────
        use reqwest::multipart;

        let mid = media_id.clone();
        let part = multipart::Part::bytes(bytes.clone())
            .mime_str(mime_type)
            .map_err(|e| ProviderError::Api(e.to_string()))?
            .file_name("media");

        self.http
            .post(api_base)
            .header("Authorization", format!("Bearer {access_token}"))
            .multipart(
                multipart::Form::new()
                    .text("command", "APPEND")
                    .text("media_id", mid.clone())
                    .text("segment_index", "0")
                    .part("media", part),
            )
            .send()
            .await?;

        // ── FINALIZE ────────────────────────────────────────
        let finalize_resp: serde_json::Value = self
            .http
            .post(api_base)
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&[
                ("command", "FINALIZE"),
                ("media_id", &mid),
            ])
            .send()
            .await?
            .json()
            .await?;

        // Check for processing_info (async video processing may be needed)
        if let Some(processing_info) = finalize_resp
            .get("processing_info")
            .and_then(|p| p.get("state"))
        {
            if processing_info == "pending" || processing_info == "in_progress" {
                // For MVP, poll briefly or accept the media ID
                tracing::warn!(
                    "Media processing still {processing_info} for media_id={media_id}"
                );
            }
        }

        Ok(media_id)
    }

    async fn upload_media(
        &self,
        access_token: &str,
        media: &[MediaAttachment],
    ) -> Result<Vec<String>, ProviderError> {
        if media.is_empty() {
            return Ok(vec![]);
        }

        let mut media_ids = Vec::with_capacity(media.len());
        for attachment in media {
            let id = self
                .upload_single_media(access_token, &attachment.url, &attachment.mime_type)
                .await?;
            media_ids.push(id);
        }

        Ok(media_ids)
    }

    // ── X API v2 methods ────────────────────────────────────────

    pub async fn get_me(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://api.twitter.com/2/users/me?user.fields=profile_image_url,description")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn user_lookup(&self, access_token: &str, user_id: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("https://api.twitter.com/2/users/{user_id}?user.fields=profile_image_url,description,public_metrics"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn user_lookup_by_username(&self, access_token: &str, username: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("https://api.twitter.com/2/users/by/username/{username}?user.fields=profile_image_url,description,public_metrics"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn home_timeline(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/timelines/reverse_chronological?max_results={max_results}&tweet.fields=created_at,public_metrics,attachments&user.fields=profile_image_url&expansions=author_id,attachments.media_keys&media.fields=url,preview_image_url"
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn user_tweets(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/tweets?max_results={max_results}&tweet.fields=created_at,public_metrics&expansions=attachments.media_keys&media.fields=url,preview_image_url"
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn tweet_detail(
        &self,
        access_token: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("https://api.twitter.com/2/tweets/{tweet_id}?tweet.fields=created_at,public_metrics,attachments,referenced_tweets&expansions=author_id,attachments.media_keys,referenced_tweets.id&user.fields=profile_image_url&media.fields=url,preview_image_url"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn search_tweets(
        &self,
        access_token: &str,
        query: &str,
        max_results: u32,
        next_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let encoded_query = urlencoding::encode(query);
        let mut url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query={encoded_query}&max_results={max_results}&tweet.fields=created_at,public_metrics,attachments&expansions=author_id,attachments.media_keys&user.fields=profile_image_url&media.fields=url,preview_image_url"
        );
        if let Some(token) = next_token {
            url.push_str(&format!("&next_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn delete_tweet(
        &self,
        access_token: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!("https://api.twitter.com/2/tweets/{tweet_id}"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    // ── Write operations (may need expanded scopes) ──

    pub async fn like_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post(format!("https://api.twitter.com/2/users/{user_id}/likes"))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&serde_json::json!({"tweet_id": tweet_id}))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn unlike_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!("https://api.twitter.com/2/users/{user_id}/likes/{tweet_id}"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn retweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post(format!("https://api.twitter.com/2/users/{user_id}/retweets"))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&serde_json::json!({"tweet_id": tweet_id}))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn unretweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!("https://api.twitter.com/2/users/{user_id}/retweets/{tweet_id}"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn bookmarks(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/bookmarks?max_results={max_results}&tweet.fields=created_at,public_metrics,attachments&expansions=author_id,attachments.media_keys&media.fields=url,preview_image_url&user.fields=profile_image_url"
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn bookmark_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post(format!("https://api.twitter.com/2/users/{user_id}/bookmarks"))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&serde_json::json!({"tweet_id": tweet_id}))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn unbookmark_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!("https://api.twitter.com/2/users/{user_id}/bookmarks/{tweet_id}"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn followers(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/followers?max_results={max_results}&user.fields=profile_image_url,description,public_metrics"
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn following(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/following?max_results={max_results}&user.fields=profile_image_url,description,public_metrics"
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn follow_user(
        &self,
        access_token: &str,
        user_id: &str,
        target_user_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post(format!("https://api.twitter.com/2/users/{user_id}/following"))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&serde_json::json!({"target_user_id": target_user_id}))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn unfollow_user(
        &self,
        access_token: &str,
        user_id: &str,
        target_user_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!("https://api.twitter.com/2/users/{user_id}/following/{target_user_id}"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }

    pub async fn list_timeline(
        &self,
        access_token: &str,
        list_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let max_results = max_results.min(100);
        let mut url = format!(
            "https://api.twitter.com/2/lists/{list_id}/tweets?max_results={max_results}&tweet.fields=created_at,public_metrics&expansions=author_id&user.fields=profile_image_url"
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized")
            || status == 401
        {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown API error");
            Err(ProviderError::Api(detail.to_string()))
        }
    }
}
