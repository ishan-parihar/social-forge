// ─── LinkedIn Page Provider ────────────────────────────────────
// Extends LinkedIn with org-level posting (companies/pages).
// Same OAuth credentials as LinkedIn but uses organization URNs.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct LinkedInPageProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl LinkedInPageProvider {
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

    /// Fetch organization logo via REST API (fallback when decoration doesn't work)
    async fn fetch_org_logo(&self, access_token: &str, org_id: &str) -> Result<String, ProviderError> {
        // Use the REST API with version header to get logoV2 URN
        let resp = self.http
            .get(format!("https://api.linkedin.com/rest/organizations/{org_id}?fields=logoV2"))
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202401")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        // logoV2.original is a URN like "urn:li:image:C560BAQ..."
        if let Some(urn) = json["logoV2"]["original"].as_str() {
            // Resolve the image URN to a download URL
            let encoded = urlencoding::encode(urn);
            let img_resp = self.http
                .get(format!("https://api.linkedin.com/rest/images/{encoded}"))
                .header("Authorization", format!("Bearer {access_token}"))
                .header("LinkedIn-Version", "202401")
                .send()
                .await?;
            let img_json: serde_json::Value = img_resp.json().await.unwrap_or_default();
            if let Some(url) = img_json["downloadUrl"].as_str() {
                return Ok(url.to_string());
            }
        }
        Err(ProviderError::Api("No logo found".into()))
    }

    fn authorize_url(&self) -> &'static str {
        "https://www.linkedin.com/oauth/v2/authorization"
    }
}

#[async_trait]
impl SocialProvider for LinkedInPageProvider {
    fn identifier(&self) -> &'static str {
        "linkedin-page"
    }

    fn name(&self) -> &'static str {
        "LinkedIn Page"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "openid".into(),
            "profile".into(),
            "email".into(),
            "w_member_social".into(),
            "rw_organization_admin".into(),
            "w_organization_social".into(),
            "r_organization_social".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        3000
    }

    fn is_between_steps(&self) -> bool { true }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Post to a LinkedIn Company Page you administer")
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

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let resp = self.http.post(self.token_url()).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;

        Ok(AuthToken {
            access_token: json["access_token"]
                .as_str()
                .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
                .to_string(),
            refresh_token: json["refresh_token"].as_str().map(String::from),
            expires_in: json["expires_in"].as_u64().map(|v| v as u32),
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    /// List organizations the user can administer
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get("https://api.linkedin.com/v2/organizationalEntityAcls?q=roleAssignee&role=ADMINISTRATOR&projection=(elements*(organizationalTarget~(localizedName,vanityName,logoV2(original~:playableStreams))))")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let elements = json["elements"].as_array().map(|a| a.to_vec()).unwrap_or_default();

        let mut pages = Vec::new();
        for e in &elements {
            let target = &e["organizationalTarget~"];
            let id = e["organizationalTarget"].as_str()
                .unwrap_or("")
                .split(':')
                .next_back()
                .unwrap_or("")
                .to_string();

            // Try decoration first, fall back to separate logo fetch
            let mut picture = target["logoV2"]["original~"]["elements"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|el| el["identifiers"].as_array())
                .and_then(|ids| ids.first())
                .and_then(|id_val| id_val["identifier"].as_str())
                .map(String::from);

            // Fallback: fetch logo via /rest/organizations endpoint
            if picture.is_none() && !id.is_empty() {
                if let Ok(logo_url) = self.fetch_org_logo(access_token, &id).await {
                    picture = Some(logo_url);
                }
            }

            pages.push(PageInfo {
                id,
                name: target["localizedName"].as_str().unwrap_or("").to_string(),
                access_token: Some(access_token.to_string()),
                picture,
                username: target["vanityName"].as_str().map(String::from),
            });
        }

        Ok(pages)
    }

    async fn fetch_page_info(&self, access_token: &str, page_id: &str) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get(format!("https://api.linkedin.com/v2/organizations/{page_id}?projection=(id,localizedName,vanityName,logoV2(original~:playableStreams))"))
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        let id_str = json["id"].as_u64()
            .map(|n| n.to_string())
            .or_else(|| json["id"].as_str().map(String::from))
            .unwrap_or_else(|| page_id.to_string());

        let mut picture = json["logoV2"]["original~"]["elements"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|el| el["identifiers"].as_array())
            .and_then(|ids| ids.first())
            .and_then(|id| id["identifier"].as_str())
            .map(String::from);

        if picture.is_none() {
            if let Ok(logo_url) = self.fetch_org_logo(access_token, page_id).await {
                picture = Some(logo_url);
            }
        }

        Ok(PageInfo {
            id: id_str,
            name: json["localizedName"].as_str().unwrap_or("").to_string(),
            access_token: Some(access_token.to_string()),
            picture,
            username: json["vanityName"].as_str().map(String::from),
        })
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
        // Resolve org ID — internal_id tells us which org to post as
        let org_id = self.resolve_org_id(access_token).await?;

        let body = serde_json::json!({
            "author": format!("urn:li:organization:{org_id}"),
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
                platform_post_url: None,
                status: "published".into(),
            });
        }

        let json: serde_json::Value = resp.json().await?;

        if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn Page publish failed")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    fn map_error(&self, body: &str, _status: u16) -> Option<String> {
        if body.contains("Unable to obtain activity") {
            Some("Unable to obtain activity. Please try again.".into())
        } else if body.contains("resource is forbidden") {
            Some("Resource is forbidden. Check your organization permissions.".into())
        } else {
            None
        }
    }

    async fn analytics(
        &self,
        access_token: &str,
        internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let org_id = if internal_id.is_empty() {
            self.resolve_org_id(access_token).await?
        } else {
            internal_id.to_string()
        };
        let org_urn = format!("urn:li:organization:{org_id}");

        let mut results = Vec::new();

        // Share statistics
        let share_url = format!(
            "https://api.linkedin.com/rest/organizationalEntityShareStatistics?q=organizationalEntity&organizationalEntity={org_urn}"
        );
        let resp = self
            .http
            .get(&share_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202601")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("LinkedIn API rate limit".into()));
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if !status.is_success() {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn share statistics error")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                if let Some(stats) = element["totalShareStatistics"].as_object() {
                    for (key, val) in stats {
                        if let Some(n) = val.as_u64() {
                            results.push(AnalyticsData {
                                label: key.clone(),
                                data: vec![AnalyticsDataPoint {
                                    total: n.to_string(),
                                    date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                }],
                                percentage_change: 0.0,
                            });
                        }
                    }
                }
            }
        }

        // Follower count
        let follower_url = format!(
            "https://api.linkedin.com/rest/networkSizes/{org_urn}?edgeType=CompanyFollowedByMember"
        );
        let resp = self
            .http
            .get(&follower_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202601")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("LinkedIn API rate limit".into()));
        }

        if status.is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(elements) = json["elements"].as_array() {
                if let Some(elem) = elements.first() {
                    if let Some(count) = elem["firstDegreeSize"].as_u64() {
                        results.push(AnalyticsData {
                            label: "followerCount".into(),
                            data: vec![AnalyticsDataPoint {
                                total: count.to_string(),
                                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                            }],
                            percentage_change: 0.0,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/rest/shares/{platform_post_id}/shareStatistics"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202601")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("LinkedIn API rate limit".into()));
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if !status.is_success() {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn post statistics error")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let mut results = Vec::new();

        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                if let Some(stats) = element["totalShareStatistics"].as_object() {
                    for (key, val) in stats {
                        if let Some(n) = val.as_u64() {
                            results.push(AnalyticsData {
                                label: key.clone(),
                                data: vec![AnalyticsDataPoint {
                                    total: n.to_string(),
                                    date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                }],
                                percentage_change: 0.0,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    // ── Import recent posts from LinkedIn Page ────────────────
    async fn get_recent_posts(
        &self,
        access_token: &str,
        internal_id: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        let page_id = if internal_id.is_empty() {
            self.resolve_org_id(access_token).await?
        } else {
            internal_id.to_string()
        };
        // Try fetching with the given page_id; if it fails, resolve the correct org ID
        let json = match self.get_page_posts(access_token, &page_id, limit).await {
            Ok(json) => json,
            Err(ProviderError::Api(msg)) if msg.contains("No virtual resource") || msg.contains("not found") => {
                tracing::warn!(
                    "LinkedIn page posts failed with '{}' for page_id={}, resolving correct org ID...",
                    msg, page_id,
                );
                let resolved_id = self.resolve_org_id(access_token).await?;
                self.get_page_posts(access_token, &resolved_id, limit).await?
            }
            Err(e) => return Err(e),
        };

        let mut posts = Vec::new();
        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                let post_urn = element["id"].as_str().unwrap_or("").to_string();
                let commentary = element["commentary"]
                    .as_object()
                    .and_then(|c| c["text"].as_str())
                    .unwrap_or("")
                    .to_string();
                let created_at = element["createdAt"].as_i64()
                    .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms))
                    .unwrap_or_else(chrono::Utc::now);

                // Extract media from the post content
                let mut media = Vec::new();
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
                    }
                }
                let post_url = Some(format!("https://www.linkedin.com/feed/update/{post_urn}"));
                posts.push(ExternalPostData {
                    platform_post_id: post_urn,
                    text: commentary,
                    author_name: None,
                    author_handle: None,
                    author_avatar: None,
                    created_at,
                    url: post_url,
                    media,
                    metadata: Some(element.clone()),
                });
            }
        }
        Ok(posts)
    }

    // ── Fetch engagement for a Page post ─────────────────────
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
            .header("LinkedIn-Version", "202601")
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
}

impl LinkedInPageProvider {
    /// Resolve a LinkedIn media URN (e.g. "urn:li:image:12345") to a direct download/playback URL.
    pub async fn resolve_media_url(&self, access_token: &str, media_urn: &str) -> Option<String> {
        let parts: Vec<&str> = media_urn.split(':').collect();
        if parts.len() < 4 { return None; }
        let asset_type = parts[2];
        let asset_id = parts[3];
        let base_url = match asset_type {
            "image" => "https://api.linkedin.com/rest/images",
            "video" => "https://api.linkedin.com/rest/videos",
            _ => return None,
        };
        let url = format!("{base_url}/{asset_id}");
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.http.get(&url)
                .header("Authorization", format!("Bearer {access_token}"))
                .header("X-Restli-Protocol-Version", "2.0.0")
                .header("LinkedIn-Version", "202601")
                .send(),
        ).await;
        match resp {
            Ok(Ok(r)) if r.status().is_success() => {
                if let Ok(json) = r.json::<serde_json::Value>().await {
                    if let Some(dl) = json.pointer("/value/downloadUrl").and_then(|v| v.as_str()) {
                        return Some(dl.to_string());
                    }
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
                }
            }
            _ => {}
        }
        None
    }

    async fn resolve_org_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let pages = self.pages(access_token).await?;
        pages.first()
            .map(|p| p.id.clone())
            .ok_or_else(|| ProviderError::Auth("No LinkedIn organizations found".into()))
    }

    pub async fn get_page_posts(&self, access_token: &str, page_id: &str, limit: u32) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.clamp(1, 100);
        let author_urn = format!("urn:li:organization:{page_id}");
        let url = format!("https://api.linkedin.com/v2/rest/posts?author={author_urn}&count={limit}");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("LinkedIn Page API error").to_string();
            tracing::warn!("LinkedIn get_page_posts HTTP {status} for page_id={page_id}: {msg}");
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn create_comment(&self, access_token: &str, post_urn: &str, page_urn: &str, message: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.linkedin.com/v2/rest/socialActions/{post_urn}/comments");
        let body = serde_json::json!({
            "actor": page_urn,
            "message": { "text": message },
            "object": post_urn
        });
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == 201 {
            let json: serde_json::Value = resp.json().await?;
            Ok(json)
        } else {
            let json: serde_json::Value = resp.json().await?;
            if status == 401 {
                Err(ProviderError::TokenExpired)
            } else {
                let msg = json["message"].as_str().unwrap_or("LinkedIn Page API error").to_string();
                Err(ProviderError::Api(msg))
            }
        }
    }
}
