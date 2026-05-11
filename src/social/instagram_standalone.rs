// ─── Instagram Standalone Provider ────────────────────────────
// Uses Instagram Basic Display API (graph.instagram.com).
// Separate OAuth flow from Facebook Graph API.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct InstagramStandaloneProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl InstagramStandaloneProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("instagram-standalone")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SocialProvider for InstagramStandaloneProvider {
    fn identifier(&self) -> &'static str {
        "instagram-standalone"
    }

    fn name(&self) -> &'static str {
        "Instagram (Standalone)"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "instagram_business_basic".into(),
            "instagram_business_content_publish".into(),
            "instagram_business_manage_comments".into(),
            "instagram_business_manage_insights".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        2200
    }

    fn needs_cron_refresh(&self) -> bool {
        true
    }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Connect a personal Instagram account (no Facebook Page required)")
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let scope = self.scopes().join(",");
        let https_uri = self.ensure_https_redirect_uri(redirect_uri);
        let params: Vec<(&str, &str)> = vec![
            ("enable_fb_login", "0"),
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", &https_uri),
            ("response_type", "code"),
            ("state", state),
            ("scope", scope.as_str()),
        ];

        let url = url::Url::parse_with_params(
            "https://www.instagram.com/oauth/authorize",
            &params,
        )
        .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;

        Ok(AuthUrlResponse { url: url.to_string() })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        let https_uri = self.ensure_https_redirect_uri(redirect_uri);
        // Step 1: Exchange code for short-lived token
        let params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &https_uri),
            ("code", code),
        ];

        let resp = self
            .http
            .post("https://api.instagram.com/oauth/access_token")
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let short_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Step 2: Exchange for long-lived token (60 days)
        let long_params: Vec<(&str, &str)> = vec![
            ("grant_type", "ig_exchange_token"),
            ("client_secret", self.client_secret.as_str()),
            ("access_token", short_token.as_str()),
        ];

        let long_resp = self
            .http
            .get("https://graph.instagram.com/access_token")
            .query(&long_params)
            .send()
            .await?;

        let long_json: serde_json::Value = long_resp.json().await?;
        let access_token = long_json["access_token"]
            .as_str()
            .unwrap_or(&short_token)
            .to_string();
        let expires_in = long_json["expires_in"].as_u64().map(|v| v as u32);

        // Get user info
        let user: serde_json::Value = self
            .http
            .get("https://graph.instagram.com/v21.0/me")
            .query(&[
                ("fields", "user_id,username,name,profile_picture_url"),
                ("access_token", access_token.as_str()),
            ])
            .send()
            .await?
            .json()
            .await?;

        let at = access_token.clone();
        Ok(AuthToken {
            access_token,
            refresh_token: Some(at),
            expires_in,
            provider_user_id: user["user_id"].as_str().unwrap_or("").to_string(),
            name: user["name"].as_str().unwrap_or("").to_string(),
            username: user["username"].as_str().unwrap_or("").to_string(),
            picture: user["profile_picture_url"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("grant_type", "ig_refresh_token"),
            ("access_token", refresh_token),
        ];

        let resp = self
            .http
            .get("https://graph.instagram.com/refresh_access_token")
            .query(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        let user: serde_json::Value = self
            .http
            .get("https://graph.instagram.com/v21.0/me")
            .query(&[
                ("fields", "user_id,username,name,profile_picture_url"),
                ("access_token", access_token.as_str()),
            ])
            .send()
            .await?
            .json()
            .await?;

        let at = access_token.clone();
        Ok(AuthToken {
            access_token,
            refresh_token: Some(at),
            expires_in,
            provider_user_id: user["user_id"].as_str().unwrap_or("").to_string(),
            name: user["name"].as_str().unwrap_or("").to_string(),
            username: user["username"].as_str().unwrap_or("").to_string(),
            picture: user["profile_picture_url"].as_str().map(String::from),
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        if post.media.is_empty() {
            return Err(ProviderError::InvalidRequest(
                "Instagram requires at least one media attachment".into(),
            ));
        }

        let ig_id = self.resolve_user_id(access_token).await?;

        // Create media container
        let is_video = post.media[0].url.contains(".mp4");
        let media_type = if is_video { "VIDEO" } else { "IMAGE" };
        let url_key = if is_video { "video_url" } else { "image_url" };

        let mut form: Vec<(&str, &str)> = vec![
            (url_key, post.media[0].url.as_str()),
            ("media_type", media_type),
            ("access_token", access_token),
        ];
        if post.media.len() == 1 {
            form.push(("caption", &post.content));
        }

        let resp = self
            .http
            .post(format!("https://graph.instagram.com/v21.0/{ig_id}/media"))
            .form(&form)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        if let Some(err) = json["error"].as_object() {
            return Err(ProviderError::Api(
                err["message"]
                    .as_str()
                    .unwrap_or("Container creation failed")
                    .to_string(),
            ));
        }

        let container_id = json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Api(format!("No container ID: {json:?}")))?
            .to_string();

        // Publish container
        let pub_form: Vec<(&str, &str)> = vec![
            ("creation_id", container_id.as_str()),
            ("access_token", access_token),
        ];

        let pub_resp = self
            .http
            .post(format!(
                "https://graph.instagram.com/v21.0/{ig_id}/media_publish"
            ))
            .form(&pub_form)
            .send()
            .await?;

        let pub_json: serde_json::Value = pub_resp.json().await?;

        if let Some(err) = pub_json["error"].as_object() {
            return Err(ProviderError::Api(
                err["message"]
                    .as_str()
                    .unwrap_or("Publish failed")
                    .to_string(),
            ));
        }

        let media_id = pub_json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Api(format!("Publish failed: {pub_json:?}")))?
            .to_string();

        let post_url = format!("https://instagram.com/p/{media_id}");
        Ok(PublishResult {
            platform_post_id: media_id,
            platform_post_url: Some(post_url),
            status: "published".into(),
        })
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Instagram Standalone does not support page management".into(),
        ))
    }
}

impl InstagramStandaloneProvider {
    fn ensure_https_redirect_uri(&self, uri: &str) -> String {
        if uri.starts_with("http://") {
            format!("https://redirectmeto.com/{}", uri)
        } else {
            uri.to_string()
        }
    }

    async fn resolve_user_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let user: serde_json::Value = self
            .http
            .get("https://graph.instagram.com/v21.0/me")
            .query(&[("fields", "user_id"), ("access_token", access_token)])
            .send()
            .await?
            .json()
            .await?;

        user["user_id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ProviderError::Auth("Could not resolve Instagram user ID".into()))
    }

    pub async fn get_media(
        &self,
        access_token: &str,
        ig_user_id: &str,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.min(100);
        let url = format!(
            "https://graph.instagram.com/v21.0/{ig_user_id}/media",
        );
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("fields", "id,caption,media_type,media_url,permalink,timestamp,like_count,comments_count"),
                ("limit", &limit.to_string()),
                ("access_token", access_token),
            ])
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Instagram API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json["error"]["message"]
                .as_str()
                .unwrap_or("Instagram API error")
                .to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_media_detail(
        &self,
        access_token: &str,
        media_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://graph.instagram.com/v21.0/{media_id}",);
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("fields", "id,caption,media_type,media_url,permalink,timestamp,username,like_count,comments_count"),
                ("access_token", access_token),
            ])
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Instagram API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json["error"]["message"]
                .as_str()
                .unwrap_or("Instagram API error")
                .to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_media_comments(
        &self,
        access_token: &str,
        media_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://graph.instagram.com/v21.0/{media_id}/comments",
        );
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("fields", "id,text,timestamp,username,like_count,replies"),
                ("access_token", access_token),
            ])
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Instagram API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json["error"]["message"]
                .as_str()
                .unwrap_or("Instagram API error")
                .to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn reply_to_comment(
        &self,
        access_token: &str,
        comment_id: &str,
        message: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://graph.instagram.com/v21.0/{comment_id}/replies",
        );
        let resp = self
            .http
            .post(&url)
            .form(&[("message", message), ("access_token", access_token)])
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Instagram API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json["error"]["message"]
                .as_str()
                .unwrap_or("Instagram API error")
                .to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn create_container(
        &self,
        access_token: &str,
        ig_user_id: &str,
        media_url: &str,
        caption: &str,
        media_type: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://graph.instagram.com/v21.0/{ig_user_id}/media",
        );
        let mut params: Vec<(&str, &str)> = vec![
            ("media_type", media_type),
            ("caption", caption),
            ("access_token", access_token),
        ];
        if media_type == "IMAGE" {
            params.push(("image_url", media_url));
        } else {
            params.push(("video_url", media_url));
        }
        let resp = self.http.post(&url).form(&params).send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Instagram API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json["error"]["message"]
                .as_str()
                .unwrap_or("Instagram API error")
                .to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn publish_container(
        &self,
        access_token: &str,
        ig_user_id: &str,
        creation_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://graph.instagram.com/v21.0/{ig_user_id}/media_publish",
        );
        let resp = self
            .http
            .post(&url)
            .form(&[("creation_id", creation_id), ("access_token", access_token)])
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Instagram API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let detail = json["error"]["message"]
                .as_str()
                .unwrap_or("Instagram API error")
                .to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn poll_container_status(
        &self,
        access_token: &str,
        creation_id: &str,
    ) -> Result<String, ProviderError> {
        let url = format!("https://graph.instagram.com/v21.0/{creation_id}",);
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "status_code"), ("access_token", access_token)])
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        if let Some(err) = json["error"].as_object() {
            let msg = err["message"]
                .as_str()
                .unwrap_or("Container status check failed");
            return Err(ProviderError::Api(msg.to_string()));
        }
        let status_code = json["status_code"]
            .as_str()
            .unwrap_or("IN_PROGRESS");
        Ok(status_code.to_string())
    }
}
