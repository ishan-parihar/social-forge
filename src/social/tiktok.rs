// ─── TikTok Provider ────────────────────────────────────────
// Uses TikTok OAuth 2.0 + TikTok Content Posting API.
// Supports: OAuth flow, user info, video upload, video publish.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct TikTokProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl TikTokProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("tiktok").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    /// Fetch the authenticated user's profile info.
    pub async fn get_user_info(
        &self,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://open.tiktokapis.com/v2/user/info/")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown TikTok API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// List the authenticated user's videos.
    pub async fn list_videos(
        &self,
        access_token: &str,
        max_count: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let count = max_count.clamp(1, 100);
        let body = serde_json::json!({
            "max_count": count,
            "cursor": 0,
        });

        let resp = self
            .http
            .post("https://open.tiktokapis.com/v2/video/list/")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown TikTok API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }
}

#[async_trait]
impl SocialProvider for TikTokProvider {
    fn identifier(&self) -> &'static str {
        "tiktok"
    }

    fn name(&self) -> &'static str {
        "TikTok"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "user.info.basic".into(),
            "video.publish".into(),
            "video.upload".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        220200960 // 210 MB max video size
    }

    fn uses_oauth(&self) -> bool {
        true
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let scope = self.scopes().join(",");
        let params: Vec<(&str, &str)> = vec![
            ("client_key", self.client_id.as_str()),
            ("scope", scope.as_str()),
            ("redirect_uri", redirect_uri),
            ("state", state),
            ("response_type", "code"),
        ];

        let url = url::Url::parse_with_params(
            "https://www.tiktok.com/v2/auth/authorize/",
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
        let params = serde_json::json!({
            "client_key": self.client_id,
            "client_secret": self.client_secret,
            "code": code,
            "grant_type": "authorization_code",
            "redirect_uri": redirect_uri,
        });

        let resp = self
            .http
            .post("https://open.tiktokapis.com/v2/oauth/token/")
            .header("Content-Type", "application/json")
            .json(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Token exchange failed")
                .to_string();
            return Err(ProviderError::Auth(msg));
        }

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);
        let open_id = json["open_id"].as_str().unwrap_or("").to_string();

        // Fetch user display info
        let user_info = self.get_user_info(&access_token).await.unwrap_or_default();

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: open_id,
            name: user_info["data"]["user"]["display_name"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            username: user_info["data"]["user"]["username"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            picture: user_info["data"]["user"]["avatar_url"]
                .as_str()
                .map(String::from),
        })
    }

    async fn refresh_token(&self, token: &str) -> Result<AuthToken, ProviderError> {
        let params = serde_json::json!({
            "client_key": self.client_id,
            "client_secret": self.client_secret,
            "grant_type": "refresh_token",
            "refresh_token": token,
        });

        let resp = self
            .http
            .post("https://open.tiktokapis.com/v2/oauth/token/")
            .header("Content-Type", "application/json")
            .json(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Token refresh failed")
                .to_string();
            return Err(ProviderError::Auth(msg));
        }

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let new_refresh = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        Ok(AuthToken {
            access_token,
            refresh_token: new_refresh.or_else(|| Some(token.to_string())),
            expires_in,
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    /// Publish a video to TikTok.
    ///
    /// Flow:
    ///   1. Download the video from the first media attachment URL.
    ///   2. Upload the video bytes to TikTok via multipart POST.
    ///   3. Create the video post with metadata.
    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // TikTok requires at least one video attachment
        let media = post.media.first().ok_or_else(|| {
            ProviderError::InvalidRequest("TikTok posts require a video attachment".into())
        })?;

        // Cap title at 150 characters (TikTok limit)
        let title = if post.content.len() > 150 {
            post.content[..150].to_string()
        } else {
            post.content.clone()
        };

        // Read privacy level from settings, defaulting to public
        let privacy_level = post
            .settings
            .get("privacy_level")
            .and_then(|v| v.as_str())
            .unwrap_or("PUBLIC_TO_EVERYONE");

        // Step 1: Download the video from the provided URL
        let video_bytes = self
            .http
            .get(&media.url)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e))?
            .bytes()
            .await
            .map_err(|e| ProviderError::Network(e))?;

        // Step 2: Upload video data to TikTok
        let file_part = reqwest::multipart::Part::bytes(video_bytes.to_vec())
            .file_name("video.mp4")
            .mime_str("video/mp4")
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let upload_form = reqwest::multipart::Form::new().part("video", file_part);

        let upload_resp = self
            .http
            .post("https://open.tiktokapis.com/v2/video/upload/")
            .header("Authorization", format!("Bearer {access_token}"))
            .multipart(upload_form)
            .send()
            .await?;

        let upload_status = upload_resp.status();
        let upload_json: serde_json::Value = upload_resp.json().await?;

        if !upload_status.is_success() {
            let msg = upload_json["error"]["message"]
                .as_str()
                .unwrap_or("Video upload failed")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let publish_id = upload_json["data"]["publish_id"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Missing publish_id from upload".into()))?
            .to_string();

        // Step 3: Create/publish the video post
        let publish_body = serde_json::json!({
            "post_info": {
                "title": title,
                "privacy_level": privacy_level,
                "disable_duet": post.settings.get("disable_duet").and_then(|v| v.as_bool()).unwrap_or(false),
                "disable_comment": post.settings.get("disable_comment").and_then(|v| v.as_bool()).unwrap_or(false),
                "disable_stitch": post.settings.get("disable_stitch").and_then(|v| v.as_bool()).unwrap_or(false),
                "brand_content": post.settings.get("brand_content").and_then(|v| v.as_bool()).unwrap_or(false),
                "brand_organic_authorization": post.settings.get("brand_organic_authorization").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "source_info": {
                "source": "FILE_UPLOAD",
                "video_upload_id": publish_id,
            },
        });

        let publish_resp = self
            .http
            .post("https://open.tiktokapis.com/v2/video/publish/")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&publish_body)
            .send()
            .await?;

        let publish_status = publish_resp.status();
        let publish_json: serde_json::Value = publish_resp.json().await?;

        if !publish_status.is_success() {
            let msg = publish_json["error"]["message"]
                .as_str()
                .unwrap_or("Video publish failed")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let post_id_val = publish_json["data"]["post_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(PublishResult {
            platform_post_url: Some(format!("https://www.tiktok.com/@i/video/{post_id_val}")),
            platform_post_id: post_id_val,
            status: "published".into(),
        })
    }

    async fn get_recent_posts(&self, access_token: &str, _internal_id: &str, limit: u32) -> Result<Vec<ExternalPostData>, ProviderError> {
        let max_count = limit.clamp(1, 100);
        let videos = self.list_videos(access_token, max_count).await?;
        let mut posts = Vec::new();

        if let Some(data) = videos["data"]["videos"].as_array() {
            for item in data {
                let video_id = item["id"].as_str().unwrap_or("").to_string();
                let title = item["title"].as_str().unwrap_or("").to_string();
                let cover_url = item["cover_image_url"].as_str().map(|s| s.to_string());
                let share_url = item["share_url"].as_str().map(|s| s.to_string());
                let create_time_ts = item["create_time"].as_i64().unwrap_or(0);

                let posted_at = chrono::DateTime::from_timestamp(create_time_ts, 0)
                    .unwrap_or_default();

                // Single media item: embed URL for iframe playback
                // Cover image URL is stored in metadata.poster_url so frontend can show it as placeholder
                let embed_url = format!("https://www.tiktok.com/embed/v2/{video_id}");
                let meta = serde_json::json!({
                    "title": title.clone(),
                    "poster_url": cover_url,
                });

                posts.push(ExternalPostData {
                    platform_post_id: video_id,
                    text: title.clone(),
                    author_name: None,
                    author_handle: None,
                    author_avatar: None,
                    media: vec![MediaAttachment {
                        url: embed_url,
                        mime_type: "text/html".into(),
                        alt: Some(title.clone()),
                        poster_url: None,
                    }],
                    created_at: posted_at,
                    url: share_url,
                    metadata: Some(meta),
                });
            }
        }

        Ok(posts)
    }

    /// Return the authenticated user as a single "page" (TikTok has no multi-page concept).
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let info = self.get_user_info(access_token).await?;
        let user = &info["data"]["user"];

        Ok(vec![PageInfo {
            id: user["open_id"].as_str().unwrap_or("").to_string(),
            name: user["display_name"].as_str().unwrap_or("").to_string(),
            access_token: Some(access_token.to_string()),
            picture: user["avatar_url"].as_str().map(String::from),
            username: user["username"].as_str().map(String::from),
        }])
    }

    /// Fetch page info by open_id. Since TikTok is single-user OAuth, returns the
    /// authenticated user's info regardless of page_id.
    async fn fetch_page_info(
        &self,
        access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let info = self.get_user_info(access_token).await?;
        let user = &info["data"]["user"];

        Ok(PageInfo {
            id: user["open_id"].as_str().unwrap_or("").to_string(),
            name: user["display_name"].as_str().unwrap_or("").to_string(),
            access_token: Some(access_token.to_string()),
            picture: user["avatar_url"].as_str().map(String::from),
            username: user["username"].as_str().map(String::from),
        })
    }

    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        _page_id: &str,
    ) -> Result<ReconnectResult, ProviderError> {
        let info = self.fetch_page_info(access_token, "").await?;
        Ok(ReconnectResult {
            id: info.id,
            name: info.name,
            access_token: info.access_token.unwrap_or_default(),
            picture: info.picture,
            username: info.username,
        })
    }
}
