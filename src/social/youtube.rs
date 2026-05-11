// ─── YouTube Provider (Stub) ───────────────────────────────────
// Uses Google OAuth 2.0 + YouTube Data API v3.
// Full implementation requires: OAuth flow, channel selection, video upload.
// Current state: Basic auth flow + info retrieval.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct YoutubeProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl YoutubeProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("youtube").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    pub async fn search_videos(
        &self,
        access_token: &str,
        query: &str,
        max_results: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let mr = max_results.clamp(1, 50).to_string();
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/search")
            .query(&[
                ("part", "snippet"),
                ("q", query),
                ("maxResults", &mr),
                ("type", "video"),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_video(
        &self,
        access_token: &str,
        video_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/videos")
            .query(&[
                ("part", "snippet,statistics,contentDetails"),
                ("id", video_id),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_playlists(
        &self,
        access_token: &str,
        channel_id: &str,
        max_results: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let mr = max_results.clamp(1, 50).to_string();
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/playlists")
            .query(&[
                ("part", "snippet,contentDetails"),
                ("channelId", channel_id),
                ("maxResults", &mr),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_playlist_items(
        &self,
        access_token: &str,
        playlist_id: &str,
        max_results: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let mr = max_results.clamp(1, 50).to_string();
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/playlistItems")
            .query(&[
                ("part", "snippet"),
                ("playlistId", playlist_id),
                ("maxResults", &mr),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_comments(
        &self,
        access_token: &str,
        video_id: &str,
        max_results: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let mr = max_results.clamp(1, 100).to_string();
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/commentThreads")
            .query(&[
                ("part", "snippet"),
                ("videoId", video_id),
                ("maxResults", &mr),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_channel_stats(
        &self,
        access_token: &str,
        channel_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/channels")
            .query(&[
                ("part", "snippet,statistics"),
                ("id", channel_id),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_analytics(
        &self,
        access_token: &str,
        channel_id: &str,
        metrics: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let ids = format!("channel=={}", channel_id);
        let resp = self
            .http
            .get("https://youtubeanalytics.googleapis.com/v2/reports")
            .query(&[
                ("ids", ids.as_str()),
                ("metrics", metrics),
                ("startDate", start_date),
                ("endDate", end_date),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_subscriptions(
        &self,
        access_token: &str,
        channel_id: &str,
        max_results: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let mr = max_results.clamp(1, 50).to_string();
        let resp = self
            .http
            .get("https://youtube.googleapis.com/youtube/v3/subscriptions")
            .query(&[
                ("part", "snippet"),
                ("channelId", channel_id),
                ("maxResults", &mr),
                ("access_token", access_token),
            ])
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
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Search for creators on a topic. Searches videos by query, groups by channel,
    /// and enriches each channel with subscriber count, email from description.
    pub async fn find_creators(&self, access_token: &str, query: &str, min_subscribers: Option<u32>, max_results: Option<u32>) -> Result<serde_json::Value, ProviderError> {
        let limit = max_results.unwrap_or(10).min(50);
        let encoded_query: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let search_url = format!(
            "https://www.googleapis.com/youtube/v3/search?part=snippet&q={}&type=video&maxResults={}&access_token={}",
            encoded_query, limit, access_token
        );
        let search_resp = self.http.get(&search_url)
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let search_status = search_resp.status();
        let search_text = search_resp.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        if !search_status.is_success() {
            let v: serde_json::Value = serde_json::from_str(&search_text).unwrap_or_default();
            return Err(ProviderError::Api(v["error"]["message"].as_str().unwrap_or(&search_text).into()));
        }
        let search_data: serde_json::Value = serde_json::from_str(&search_text).unwrap_or_default();

        let mut channel_ids: Vec<String> = Vec::new();
        if let Some(items) = search_data["items"].as_array() {
            for item in items {
                if let Some(ch_id) = item["snippet"]["channelId"].as_str() {
                    let id = ch_id.to_string();
                    if !channel_ids.contains(&id) {
                        channel_ids.push(id);
                    }
                }
            }
        }

        if channel_ids.is_empty() {
            return Ok(serde_json::json!({"creators": [], "total_videos": 0}));
        }

        let ids_param = channel_ids.join(",");
        let stats_url = format!(
            "https://www.googleapis.com/youtube/v3/channels?part=snippet,statistics&id={}&access_token={}",
            ids_param, access_token
        );
        let stats_resp = self.http.get(&stats_url)
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let stats_status = stats_resp.status();
        let stats_text = stats_resp.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        if !stats_status.is_success() {
            let v: serde_json::Value = serde_json::from_str(&stats_text).unwrap_or_default();
            return Err(ProviderError::Api(v["error"]["message"].as_str().unwrap_or(&stats_text).into()));
        }
        let stats_data: serde_json::Value = serde_json::from_str(&stats_text).unwrap_or_default();

        let min_subs = min_subscribers.unwrap_or(0) as i64;
        let mut creators = Vec::new();
        if let Some(channels) = stats_data["items"].as_array() {
            for ch in channels {
                let sub_count: i64 = ch["statistics"]["subscriberCount"].as_str()
                    .and_then(|s| s.parse().ok()).unwrap_or(0);
                if sub_count >= min_subs {
                    let description = ch["snippet"]["description"].as_str().unwrap_or("");
                    let email = description.split_whitespace()
                        .find(|w| w.contains('@') && w.contains('.'))
                        .map(|e| e.trim_end_matches('.').trim_end_matches(',').to_string());
                    creators.push(serde_json::json!({
                        "channel_id": ch["id"],
                        "title": ch["snippet"]["title"],
                        "description": ch["snippet"]["description"],
                        "subscriber_count": sub_count,
                        "video_count": ch["statistics"]["videoCount"],
                        "view_count": ch["statistics"]["viewCount"],
                        "thumbnail": ch["snippet"]["thumbnails"]["default"]["url"],
                        "email": email,
                        "country": ch["snippet"]["country"],
                        "published_at": ch["snippet"]["publishedAt"]
                    }));
                }
            }
        }

        Ok(serde_json::json!({
            "creators": creators,
            "total_videos": search_data["pageInfo"]["totalResults"],
            "query": query
        }))
    }
}

#[async_trait]
impl SocialProvider for YoutubeProvider {
    fn identifier(&self) -> &'static str {
        "youtube"
    }

    fn name(&self) -> &'static str {
        "YouTube"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "https://www.googleapis.com/auth/youtube".into(),
            "https://www.googleapis.com/auth/youtube.upload".into(),
            "https://www.googleapis.com/auth/youtube.force-ssl".into(),
            "https://www.googleapis.com/auth/userinfo.profile".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        5000
    }

    fn is_between_steps(&self) -> bool {
        true
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        // Google OAuth 2.0
        let scope = self.scopes().join(" ");
        let params: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("scope", scope.as_str()),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ];

        let url = url::Url::parse_with_params(
            "https://accounts.google.com/o/oauth2/v2/auth",
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
        let params: Vec<(&str, &str)> = vec![
            ("code", code),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        // Get user info
        let user: serde_json::Value = self
            .http
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json()
            .await?;

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["name"].as_str().unwrap_or("").to_string(),
            username: user["email"].as_str().unwrap_or("").to_string(),
            picture: user["picture"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        Ok(AuthToken {
            access_token,
            refresh_token: Some(refresh_token.to_string()),
            expires_in,
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    /// List channels for page selection
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get("https://www.googleapis.com/youtube/v3/channels")
            .query(&[
                ("part", "snippet"),
                ("mine", "true"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let items = json["items"].as_array().cloned().unwrap_or_default();

        Ok(items
            .iter()
            .map(|item| PageInfo {
                id: item["id"].as_str().unwrap_or("").to_string(),
                name: item["snippet"]["title"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                access_token: Some(access_token.to_string()),
                picture: item["snippet"]["thumbnails"]["default"]["url"]
                    .as_str()
                    .map(String::from),
                username: item["snippet"]["customUrl"]
                    .as_str()
                    .map(String::from),
            })
            .collect())
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get("https://www.googleapis.com/youtube/v3/channels")
            .query(&[
                ("part", "snippet"),
                ("id", page_id),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        if let Some(item) = json["items"].as_array().and_then(|a| a.first()) {
            Ok(PageInfo {
                id: item["id"].as_str().unwrap_or("").to_string(),
                name: item["snippet"]["title"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                access_token: Some(access_token.to_string()),
                picture: item["snippet"]["thumbnails"]["default"]["url"]
                    .as_str()
                    .map(String::from),
                username: item["snippet"]["customUrl"]
                    .as_str()
                    .map(String::from),
            })
        } else {
            Err(ProviderError::Api("YouTube channel not found".into()))
        }
    }

    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        page_id: &str,
    ) -> Result<ReconnectResult, ProviderError> {
        let info = self.fetch_page_info(access_token, page_id).await?;
        Ok(ReconnectResult {
            id: info.id,
            name: info.name,
            access_token: info.access_token.unwrap_or_default(),
            picture: info.picture,
            username: info.username,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        if post.media.is_empty() {
            return Err(ProviderError::InvalidRequest(
                "YouTube video upload requires at least one media attachment (video file)."
                    .into(),
            ));
        }

        let media = &post.media[0];

        let video_resp = self.http.get(&media.url).send().await?;
        if !video_resp.status().is_success() {
            return Err(ProviderError::Api(format!(
                "Failed to download video from media URL: HTTP {}",
                video_resp.status()
            )));
        }
        let video_bytes = video_resp.bytes().await?.to_vec();

        let mime_type = if media.mime_type.is_empty() {
            "video/*"
        } else {
            &media.mime_type
        };

        // YouTube title max is 100 chars
        let title: String = post.content.chars().take(100).collect();
        let description = &post.content;

        let privacy_status = post
            .settings
            .get("privacyStatus")
            .and_then(|v| v.as_str())
            .unwrap_or("private");

        let metadata = serde_json::json!({
            "snippet": {
                "title": title,
                "description": description,
            },
            "status": {
                "privacyStatus": privacy_status,
            }
        });

        // ── Step 1: Initialize resumable upload ────────────────────────────
        let init_resp = self
            .http
            .post(
                "https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status",
            )
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .header("X-Upload-Content-Type", mime_type)
            .header(
                "X-Upload-Content-Length",
                video_bytes.len().to_string(),
            )
            .body(
                serde_json::to_string(&metadata)
                    .map_err(|e| ProviderError::Api(e.to_string()))?,
            )
            .send()
            .await?;

        let init_status = init_resp.status();
        if init_status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if !init_status.is_success() {
            let err_body: serde_json::Value =
                init_resp.json().await.unwrap_or_default();
            let msg = err_body["error"]["message"]
                .as_str()
                .unwrap_or("Failed to initialize resumable upload")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let upload_url = init_resp
            .headers()
            .get(reqwest::header::LOCATION)
            .ok_or_else(|| {
                ProviderError::Api(
                    "No Location header in resumable upload response".into(),
                )
            })?
            .to_str()
            .map_err(|e| {
                ProviderError::Api(format!("Invalid Location header: {e}"))
            })?
            .to_string();

        // ── Step 2: Upload the video binary data ───────────────────────────
        let upload_resp = self
            .http
            .put(&upload_url)
            .header("Content-Type", mime_type)
            .header("Content-Length", video_bytes.len().to_string())
            .body(video_bytes)
            .send()
            .await?;

        let upload_status = upload_resp.status();
        if upload_status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if !upload_status.is_success() {
            let err_body: serde_json::Value =
                upload_resp.json().await.unwrap_or_default();
            let msg = err_body["error"]["message"]
                .as_str()
                .unwrap_or("Failed to upload video to YouTube")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let json: serde_json::Value = upload_resp.json().await?;
        let video_id = json["id"].as_str().ok_or_else(|| {
            ProviderError::Api("YouTube did not return a video ID".into())
        })?;

        Ok(PublishResult {
            platform_post_id: video_id.to_string(),
            platform_post_url: Some(format!(
                "https://www.youtube.com/watch?v={video_id}"
            )),
            status: "published".into(),
        })
    }
}
