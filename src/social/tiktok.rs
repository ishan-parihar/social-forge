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

    /// Find one video's metrics in a `/v2/video/list/` response by id.
    /// The Display API returns videos newest-first, so the first page is where a
    /// just-published post lives.
    fn find_video<'a>(
        list: &'a serde_json::Value,
        video_id: &str,
    ) -> Option<&'a serde_json::Value> {
        list["data"]["videos"]
            .as_array()?
            .iter()
            .find(|v| v["id"].as_str() == Some(video_id))
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
            // Display API read scope: backs list_videos(), which get_recent_posts()
            // and the analytics/engagement paths all depend on.
            "video.list".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        220200960 // 210 MB max video size
    }

    fn uses_oauth(&self) -> bool {
        true
    }

    /// TikTok access tokens live 24 hours (or only 7 days once the app is
    /// public), so a missed refresh takes publishing offline for a day.
    fn needs_cron_refresh(&self) -> bool {
        true
    }

    /// TikTok-specific user-facing error copy (v25 §2 row 9).
    fn map_error(&self, body: &str, status: u16) -> Option<String> {
        if body.contains("DUPLICATE_CONTENT") || body.contains("duplicate content") {
            return Some(
                "TikTok rejected this video as duplicate content. TikTok hashes the audio \
                 track, so re-uploading the same clip fails even with new captions — change \
                 the video or use TikTok's repost flow instead."
                    .into(),
            );
        }
        if body.contains("unauthorized.creator_ttl") || body.contains("creator_ttl") {
            return Some(
                "TikTok rejected the video because the source URL has been posted to before. \
                 TikTok blocks re-uploads from the same URL — serve the file from a fresh URL."
                    .into(),
            );
        }
        match classify_error(body, status) {
            Some(ErrorKind::RateLimited) => Some(
                "TikTok rate limit hit. The Content Posting API allows roughly 1 000 video \
                 publishes per day per user — wait for the reset or lower the publish rate."
                    .into(),
            ),
            Some(ErrorKind::MediaRejected) => Some(
                "TikTok rejected the video. Uploads must be MP4 under 210 MB, 3-10 minutes \
                 long, H.264 with AAC audio, fetched over HTTP from a public URL."
                    .into(),
            ),
            Some(ErrorKind::AuthExpired) => Some(
                "TikTok access token expired or was revoked. TikTok tokens last 24 hours and \
                 Forge refreshes them automatically — reconnect if the refresh is rejected."
                    .into(),
            ),
            Some(ErrorKind::Forbidden) => Some(
                "TikTok denied this request (HTTP 403). The app needs an approved \
                 video.publish scope, an audit note, and the account must not be under 18."
                    .into(),
            ),
            Some(ErrorKind::NotFound) => Some(
                "TikTok video or user not found — it may be private, deleted, or the app is \
                 not approved for the Content Posting API."
                    .into(),
            ),
            Some(ErrorKind::InvalidRequest) => Some(
                "TikTok rejected the request as invalid. Check that exactly one MP4 is \
                 attached, the title is within limits, and privacy_level is a value the app \
                 is authorised to use."
                    .into(),
            ),
            _ => None,
        }
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
                .unwrap_or("Video upload failed");
            let code = upload_json["error"]["code"].as_u64().unwrap_or(0);
            let raw = format!("TikTok video upload failed (HTTP {upload_status}, code {code}): {msg}");
            return Err(ProviderError::Api(friendly_error(
                self.map_error(msg, upload_status.as_u16()),
                &raw,
            )));
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
                .unwrap_or("Video publish failed");
            let code = publish_json["error"]["code"].as_u64().unwrap_or(0);
            let raw = format!("TikTok video publish failed (HTTP {publish_status}, code {code}): {msg}");
            return Err(ProviderError::Api(friendly_error(
                self.map_error(msg, publish_status.as_u16()),
                &raw,
            )));
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

    /// Per-post counts (likes, comments, shares, views) for one video.
    ///
    /// The open API has no per-video analytics endpoint, so the counts come from
    /// the Display API `video.list` feed.
    async fn get_post_engagement(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        // ponytail: first page only (100 newest). Covers any post the scheduler
        // published; a post that has fallen out of the window reports None rather
        // than a wrong zero. Page with data.cursor if that ever matters.
        let list = self.list_videos(access_token, 100).await?;
        Ok(Self::find_video(&list, platform_post_id).cloned())
    }

    /// Account-level analytics for the dashboard.
    ///
    /// `user.info.basic` returns lifetime counters only, so each series is a
    /// single point dated today and `days` carries no meaning.
    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let info = self.get_user_info(access_token).await?;
        let user = &info["data"]["user"];
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        Ok(metrics_series(
            user,
            &today,
            &[
                ("Followers", "follower_count"),
                ("Following", "following_count"),
                ("Likes", "likes_count"),
                ("Videos", "video_count"),
            ],
        ))
    }

    /// Per-post analytics, rendered from the same Display API counts as
    /// `get_post_engagement` — there is no separate analytics endpoint.
    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let Some(video) = self.get_post_engagement(access_token, platform_post_id).await? else {
            return Ok(vec![]);
        };
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        Ok(metrics_series(
            &video,
            &today,
            &[
                ("Views", "view_count"),
                ("Likes", "like_count"),
                ("Comments", "comment_count"),
                ("Shares", "share_count"),
            ],
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::parse_engagement_data;

    #[test]
    fn parse_should_map_tiktok_display_api_counts() {
        let raw = serde_json::json!({
            "id": "7300000000000000000",
            "view_count": 1200,
            "like_count": 42,
            "comment_count": 12,
            "share_count": 5
        });
        let e = parse_engagement_data("tiktok", raw);
        assert_eq!(e.views, 1200);
        assert_eq!(e.likes, 42);
        assert_eq!(e.comments, 12);
        assert_eq!(e.shares, 5);
    }

    #[test]
    fn parse_should_read_string_encoded_tiktok_counts() {
        // `/v2/video/query/?fields=…` can return counts as strings.
        let raw = serde_json::json!({ "like_count": "8", "view_count": "90" });
        let e = parse_engagement_data("tiktok", raw);
        assert_eq!(e.views, 90);
        assert_eq!(e.likes, 8);
    }

    #[test]
    fn parse_should_default_missing_tiktok_metrics_to_zero() {
        let e = parse_engagement_data("tiktok", serde_json::json!({ "like_count": 3 }));
        assert_eq!(e.likes, 3);
        assert_eq!(e.views, 0);
        assert_eq!(e.shares, 0);
    }

    #[test]
    fn find_video_should_match_by_id() {
        let list = serde_json::json!({
            "data": { "videos": [ { "id": "111", "like_count": 1 }, { "id": "222", "like_count": 2 } ] }
        });
        let found = TikTokProvider::find_video(&list, "222").expect("video 222 must be found");
        assert_eq!(found["like_count"], 2);
    }

    #[test]
    fn find_video_should_return_none_for_unknown_id() {
        let list = serde_json::json!({ "data": { "videos": [ { "id": "111" } ] } });
        assert!(TikTokProvider::find_video(&list, "999").is_none());
    }

    #[test]
    fn metrics_series_should_skip_tiktok_counters_absent_from_response() {
        // Display API omits counters for private/under-review videos.
        let video = serde_json::json!({ "id": "1", "like_count": 4 });
        let series = metrics_series(
            &video,
            "2026-09-30",
            &[("Views", "view_count"), ("Likes", "like_count"), ("Shares", "share_count")],
        );
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].label, "Likes");
        assert_eq!(series[0].data[0].total, "4");
    }

    // ── B4: needs_cron_refresh + map_error fixtures ────────

    fn provider() -> TikTokProvider {
        TikTokProvider::new(&crate::social::test_config())
    }

    #[test]
    fn tiktok_requests_cron_refresh() {
        assert!(provider().needs_cron_refresh());
    }

    #[test]
    fn map_error_explains_duplicate_content() {
        let msg = provider().map_error("DUPLICATE_CONTENT", 400).unwrap();
        assert!(msg.contains("duplicate content"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_creator_ttl_rejection() {
        let msg = provider()
            .map_error("The video has been posted before (unauthorized.creator_ttl)", 400)
            .unwrap();
        assert!(msg.contains("fresh URL"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_rate_limit_on_429() {
        let msg = provider().map_error("Too many requests", 429).unwrap();
        assert!(msg.contains("rate limit"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_rejected_media() {
        let msg = provider().map_error("The video upload was rejected", 422).unwrap();
        assert!(msg.contains("210 MB"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_expired_token_on_401() {
        let msg = provider().map_error("", 401).unwrap();
        assert!(msg.contains("24 hours"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_unapproved_app_on_403() {
        let msg = provider().map_error("", 403).unwrap();
        assert!(msg.contains("video.publish"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_missing_video_on_404() {
        let msg = provider().map_error("", 404).unwrap();
        assert!(msg.contains("not found"), "got: {msg}");
    }

    #[test]
    fn map_error_explains_invalid_request_on_400() {
        let msg = provider().map_error("", 400).unwrap();
        assert!(msg.contains("privacy_level"), "got: {msg}");
    }

    #[test]
    fn map_error_returns_none_for_server_error() {
        assert!(provider().map_error("Internal Server Error", 503).is_none());
    }
}
