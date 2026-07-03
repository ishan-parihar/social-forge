// ─── Facebook Provider ────────────────────────────────────────
// OAuth 2.0 + Graph API. Posts to Facebook Page (not personal timeline).

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct FacebookProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl FacebookProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("facebook")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn graph_url(&self) -> &'static str {
        "https://graph.facebook.com/v21.0"
    }
}

#[async_trait]
impl SocialProvider for FacebookProvider {
    fn identifier(&self) -> &'static str {
        "facebook"
    }

    fn name(&self) -> &'static str {
        "Facebook"
    }

    fn is_between_steps(&self) -> bool { true }

    fn scopes(&self) -> Vec<String> {
        vec![
            "pages_show_list".into(),
            "pages_read_engagement".into(),
            "pages_manage_posts".into(),
            "business_management".into(),
            "pages_manage_engagement".into(),
            "pages_manage_metadata".into(),
            "pages_read_user_content".into(),
            "public_profile".into(),
            "read_insights".into(),
            "pages_messaging".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        63206
    }

    fn validate_media(&self, post: &PostContent) -> Result<(), String> {
        super::validate_media_limits(self.identifier(), post)
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let scope = self.scopes().join(",");
        let params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("scope", scope.as_str()),
            ("state", state),
            ("response_type", "code"),
        ];

        let url = url::Url::parse_with_params(
            "https://www.facebook.com/v21.0/dialog/oauth",
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
        // Exchange code for access token
        let token_params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("code", code),
        ];

        let token_resp = self
            .http
            .get("https://graph.facebook.com/v21.0/oauth/access_token")
            .query(&token_params)
            .send()
            .await?;

        let token_json: serde_json::Value = token_resp.json().await?;
        let short_token = token_json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Exchange short-lived token for long-lived token
        let long_params: Vec<(&str, &str)> = vec![
            ("grant_type", "fb_exchange_token"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("fb_exchange_token", short_token.as_str()),
        ];

        let long_resp = self
            .http
            .get("https://graph.facebook.com/v21.0/oauth/access_token")
            .query(&long_params)
            .send()
            .await?;

        let long_json: serde_json::Value = long_resp.json().await?;
        let access_token = long_json["access_token"]
            .as_str()
            .unwrap_or(&short_token)
            .to_string();
        let expires_in = long_json["expires_in"].as_u64().map(|v| v as u32);

        // Get Facebook user info (name + picture) for the root integration
        let me: serde_json::Value = self
            .http
            .get(format!("{}/me?fields=id,name,picture.type(large)", self.graph_url()))
            .query(&[("access_token", &access_token)])
            .send()
            .await?
            .json()
            .await?;

        let user_id = me["id"].as_str().unwrap_or("me").to_string();
        let user_name = me["name"].as_str().unwrap_or("Facebook User").to_string();
        let user_pic = me["picture"]["data"]["url"].as_str().map(String::from);

        tracing::info!(
            "Facebook OAuth: user '{}' ({}). Use available-pages + connect-page to link specific pages.",
            user_name, user_id
        );

        // Return the user-level token. The caller (is_between_steps = true) will
        // store this as a root integration. The user then calls:
        //   GET  /api/integrations/{id}/available-pages  — list all pages
        //   POST /api/integrations/{parent_id}/connect-page/{page_id} — connect a page
        return Ok(AuthToken {
            access_token,  // user-level token for listing pages via me/accounts
            refresh_token: None,
            expires_in,
            provider_user_id: user_id,
            name: user_name.clone(),
            username: user_name,
            picture: user_pic,
        });
    }

    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Facebook long-lived tokens last 60 days. Reconnect the channel.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Use the page-scoped token's associated page ID for posting.
        // The page_id is stored in the token itself or resolved from /me/accounts.
        let page_id = self.resolve_page_id(access_token).await?;

        let params = vec![
            ("message", post.content.as_str()),
            ("access_token", access_token),
        ];

        let resp = self
            .http
            .post(format!("{}/{page_id}/feed", self.graph_url()))
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 200 {
            let post_id = json["id"].as_str().unwrap_or("").to_string();
            Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: None,
                status: "published".into(),
            })
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"]
                    .as_str()
                    .unwrap_or("Facebook publish failed")
                    .to_string(),
            ))
        }
    }

    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let fields = "id,access_token,username,name,picture.type(large)";
        let mut pages_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();

        // Paginated fetch helper
        async fn collect_pages(
            http: &reqwest::Client,
            url: &str,
            access_token: &str,
            map: &mut std::collections::HashMap<String, serde_json::Value>,
        ) -> Result<(), ProviderError> {
            let mut current = url.to_string();
            loop {
                let resp = http
                    .get(&current)
                    .query(&[("access_token", access_token)])
                    .send()
                    .await?;

                let json: serde_json::Value = resp.json().await?;

                if let Some(err) = json["error"].as_object() {
                    tracing::warn!("collect_pages skipped ({:?}): {}", url, err["message"].as_str().unwrap_or(""));
                    return Ok(());
                }

                if let Some(data) = json["data"].as_array() {
                    for page in data {
                        if let Some(pid) = page["id"].as_str() {
                            map.entry(pid.to_string()).or_insert_with(|| page.clone());
                        }
                    }
                }

                if let Some(next) = json["paging"]["next"].as_str() {
                    current = next.to_string();
                } else {
                    break;
                }
            }
            Ok(())
        }

        // Phase 1: /me/accounts — pages user selected in OAuth
        let me_url = format!(
            "{}/me/accounts?fields={}&limit=100",
            self.graph_url(),
            fields
        );
        collect_pages(&self.http, &me_url, access_token, &mut pages_map).await?;

        // Phase 2: /me/businesses → owned_pages + client_pages — Business Manager pages
        let biz_resp = self
            .http
            .get(format!("{}/me/businesses", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await?;

        let biz_json: serde_json::Value = biz_resp.json().await?;

        if biz_json["error"].is_null() {
            if let Some(businesses) = biz_json["data"].as_array() {
                for biz in businesses {
                    let biz_id = match biz["id"].as_str() {
                        Some(id) if !id.is_empty() => id,
                        _ => continue,
                    };

                    for endpoint in &["owned_pages", "client_pages"] {
                        let biz_url = format!(
                            "{}/{biz_id}/{endpoint}?fields={fields}&limit=100",
                            self.graph_url(),
                        );
                        if let Err(e) = collect_pages(&self.http, &biz_url, access_token, &mut pages_map).await {
                            tracing::warn!("Business Manager {endpoint} for {biz_id}: {e:?}");
                        }
                    }
                }
            }
        } else {
            tracing::debug!("/me/businesses not available: {:?}", biz_json["error"]["message"]);
        }

        let mut result: Vec<PageInfo> = pages_map
            .into_values()
            .map(|page| PageInfo {
                id: page["id"].as_str().unwrap_or("").to_string(),
                name: page["name"].as_str().unwrap_or("").to_string(),
                access_token: page["access_token"].as_str().map(String::from),
                picture: page["picture"]["data"]["url"].as_str().map(String::from),
                username: page["username"].as_str().map(String::from),
            })
            .collect();

        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let page_id = self.resolve_page_id(access_token).await?;

        let since = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string();
        let until = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let json = self.get_page_insights(
            access_token,
            &page_id,
            "page_impressions,page_engaged_users,page_fans",
            "day",
            Some(&since),
            Some(&until),
        ).await?;

        let mut result = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for entry in data {
                let name = entry["name"].as_str().unwrap_or("unknown").to_string();
                let mut points = Vec::new();
                if let Some(values) = entry["values"].as_array() {
                    for v in values {
                        points.push(AnalyticsDataPoint {
                            total: v["value"].as_i64().unwrap_or(0).to_string(),
                            date: v["end_time"].as_str().unwrap_or("").to_string(),
                        });
                    }
                }
                result.push(AnalyticsData {
                    label: name,
                    data: points,
                    percentage_change: 0.0,
                });
            }
        }

        Ok(result)
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let url = format!("{}/{platform_post_id}/insights", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("metric", "post_impressions,post_engaged_users,post_reactions_by_type_total"),
                ("period", "lifetime"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            if status == 429 {
                return Err(ProviderError::RateLimited("Facebook API rate limit".into()));
            } else if status == 401 {
                return Err(ProviderError::TokenExpired);
            } else {
                return Err(ProviderError::Api(
                    json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
                ));
            }
        }

        let mut result = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for entry in data {
                let name = entry["name"].as_str().unwrap_or("unknown").to_string();
                let mut points = Vec::new();
                if let Some(values) = entry["values"].as_array() {
                    for v in values {
                        points.push(AnalyticsDataPoint {
                            total: v["value"].as_i64().unwrap_or(0).to_string(),
                            date: v["end_time"].as_str().unwrap_or("").to_string(),
                        });
                    }
                }
                result.push(AnalyticsData {
                    label: name,
                    data: points,
                    percentage_change: 0.0,
                });
            }
        }

        Ok(result)
    }

    async fn get_recent_posts(
        &self,
        access_token: &str,
        _internal_id: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        // Use _internal_id as the page_id if provided (from a connected page integration),
        // otherwise fall back to resolving from the token.
        let page_id = if !_internal_id.is_empty() {
            _internal_id.to_string()
        } else {
            self.resolve_page_id(access_token).await?
        };

        // Fetch page info for author details (name, handle, avatar)
        let page_info = self.fetch_page_info(access_token, &page_id).await.ok();
        let page_name = page_info.as_ref().map(|p| p.name.clone());
        let page_handle = page_info.as_ref().and_then(|p| p.username.clone());
        let page_avatar = page_info.as_ref().and_then(|p| p.picture.clone());

        let json = self.get_page_feed(access_token, &page_id, limit, None, None).await?;

        let mut posts = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for item in data {
                let post_id = item["id"].as_str().unwrap_or("").to_string();
                let message = item["message"].as_str().unwrap_or("").to_string();
                let story = item["story"].as_str().map(|s| s.to_string());
                let content = if message.is_empty() { story.unwrap_or_default() } else { message };
                let created_time = item["created_time"].as_str()
                    .map(crate::social::common::parse_timestamp)
                    .unwrap_or_else(chrono::Utc::now);

                // Build media vector — for video attachments, make a separate API call
                // to get the actual playable video URL instead of the thumbnail.
                let mut media = Vec::new();
                if let Some(arr) = item["attachments"]["data"].as_array() {
                    for a in arr {
                        let attach_type = a["type"].as_str().unwrap_or("");

                        // Handle album/carousel → extract individual items from subattachments
                        if attach_type == "album" || attach_type == "multiple" {
                            if let Some(children) = a["subattachments"]["data"].as_array() {
                                for child in children {
                                    if let Some(m) = Self::extract_single_media(child) {
                                        media.push(m);
                                    }
                                }
                            }
                            continue;
                        }

                        // Handle video: extract target.id and make a dedicated API call
                        // to get the actual playable video URL.
                        if attach_type.contains("video") || attach_type == "animated_video" {
                            if let Some(video_id) = a["target"]["id"].as_str() {
                                match self.get_video_source(access_token, video_id).await {
                                    Ok(url_str) => {
                                        media.push(MediaAttachment {
                                            url: url_str,
                                            mime_type: "video/mp4".into(),
                                            alt: None,
                                            poster_url: None,
                                        });
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to fetch video source for {}: {e}",
                                            video_id
                                        );
                                        // Fallback: use extract_single_media which will try media.source
                                        if let Some(m) = Self::extract_single_media(a) {
                                            media.push(m);
                                        }
                                    }
                                }
                            } else {
                                // No target.id — fallback to extract_single_media
                                if let Some(m) = Self::extract_single_media(a) {
                                    media.push(m);
                                }
                            }
                        } else {
                            // Handle all other types (photo, share, etc.)
                            if let Some(m) = Self::extract_single_media(a) {
                                media.push(m);
                            }
                        }
                    }
                }

                let permalink = item["permalink_url"].as_str().map(String::from);

                posts.push(ExternalPostData {
                    platform_post_id: post_id,
                    text: content,
                    author_name: page_name.clone(),
                    author_handle: page_handle.clone(),
                    author_avatar: page_avatar.clone(),
                    created_at: created_time,
                    url: permalink,
                    media,
                    metadata: None,
                });
            }
        }
        Ok(posts)
    }

    async fn get_post_engagement(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        // get_page_post already requests comments.summary(true),reactions.summary(true)
        let json = self.get_page_post(access_token, platform_post_id).await?;
        Ok(Some(json))
    }

    async fn get_post_comments(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<CommentData>, ProviderError> {
        let json = self.get_post_comments_raw(access_token, platform_post_id).await?;

        let mut comments = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for item in data {
                let id = item["id"].as_str().unwrap_or("").to_string();
                let text = item["message"].as_str().unwrap_or("").to_string();
                let created_at = item["created_time"]
                    .as_str()
                    .map(crate::social::common::parse_timestamp)
                    .unwrap_or_else(chrono::Utc::now);

                let author_name = item["from"]["name"].as_str().map(String::from);
                let author_avatar = None; // Facebook comments don't include avatar in basic fields

                // Parse nested replies if present
                let replies = if let Some(reply_data) = item["comments"]["data"].as_array() {
                    reply_data.iter().filter_map(|r| {
                        let rid = r["id"].as_str()?;
                        let rtext = r["message"].as_str().unwrap_or("");
                        let rcreated = r["created_time"]
                            .as_str()
                            .map(crate::social::common::parse_timestamp)
                            .unwrap_or_else(chrono::Utc::now);
                        let rauthor_name = r["from"]["name"].as_str().map(String::from);
                        Some(CommentData {
                            id: rid.to_string(),
                            author_name: rauthor_name,
                            author_avatar: None,
                            text: rtext.to_string(),
                            created_at: rcreated,
                            like_count: 0,
                            replies: vec![],
                        })
                    }).collect()
                } else {
                    vec![]
                };

                comments.push(CommentData {
                    id,
                    author_name,
                    author_avatar,
                    text,
                    created_at,
                    like_count: 0, // Facebook comments API doesn't include like count by default
                    replies,
                });
            }
        }
        Ok(comments)
    }

        async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/{page_id}?fields=id,name,username,picture.type(large)", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        Ok(PageInfo {
            id: json["id"].as_str().unwrap_or("").to_string(),
            name: json["name"].as_str().unwrap_or("").to_string(),
            access_token: None,
            picture: json["picture"]["data"]["url"].as_str().map(String::from),
            username: json["username"].as_str().map(String::from),
        })
    }

    fn resolve_media_url(&self, attachment: &MediaAttachment, app_url: &str) -> MediaAttachment {
        if attachment.url.starts_with("/api/media/") || attachment.url.starts_with("/media/") {
            MediaAttachment {
                url: format!("{}{}", app_url.trim_end_matches('/'), attachment.url),
                ..attachment.clone()
            }
        } else {
            attachment.clone()
        }
    }
}

impl FacebookProvider {
    /// Resolve the page ID associated with a page-scoped access token.
    async fn resolve_page_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let me: serde_json::Value = self
            .http
            .get(format!("{}/me/accounts", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await?
            .json()
            .await?;

        let pages = me["data"].as_array()
            .ok_or_else(|| ProviderError::Auth("No pages found for token. Ensure you have a Facebook Page.".into()))?;

        pages.first()
            .and_then(|page| page["id"].as_str().map(String::from))
            .ok_or_else(|| ProviderError::Auth("Could not resolve page ID from token.".into()))
    }
}

// ── Inherent Graph API Methods ──────────────────────────────────
impl FacebookProvider {
    /// Get the page's feed (posts).
    pub async fn get_page_feed(
        &self, access_token: &str, page_id: &str, limit: u32,
        since: Option<&str>, until: Option<&str>
    ) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.min(100);
        let mut url = format!(
            "{}/{page_id}/feed?fields=message,created_time,story,permalink_url,attachments{{media{{source,image{{src}}}},type,url,title,target{{id}},subattachments{{media{{source,image{{src}}}},type,url,title,target{{id}}}}}}&limit={limit}",
            self.graph_url()
        );
        if let Some(s) = since {
            url.push_str(&format!("&since={s}"));
        }
        if let Some(u) = until {
            url.push_str(&format!("&until={u}"));
        }
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Get a single post by ID.
    pub async fn get_page_post(
        &self, access_token: &str, post_id: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "{}/{post_id}?fields=id,message,created_time,permalink_url,comments.summary(true),reactions.summary(true)",
            self.graph_url()
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Get comments on a post (raw Graph API response).
    pub async fn get_post_comments_raw(
        &self, access_token: &str, post_id: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "{}/{post_id}/comments?fields=id,message,from,created_time",
            self.graph_url()
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Create a text/link post on a page.
    pub async fn create_post(
        &self, access_token: &str, page_id: &str, message: &str, link: Option<&str>
    ) -> Result<serde_json::Value, ProviderError> {
        let mut params: Vec<(&str, &str)> = vec![
            ("message", message),
            ("access_token", access_token),
        ];
        if let Some(l) = link {
            params.push(("link", l));
        }
        let resp = self.http.post(format!("{}/{page_id}/feed", self.graph_url()))
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Create a photo post on a page.
    pub async fn create_photo_post(
        &self, access_token: &str, page_id: &str, url: &str, caption: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("url", url),
            ("caption", caption),
            ("access_token", access_token),
        ];
        let resp = self.http.post(format!("{}/{page_id}/photos", self.graph_url()))
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Create a video post on a page.
    pub async fn create_video_post(
        &self, access_token: &str, page_id: &str, file_url: &str, title: &str, description: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("file_url", file_url),
            ("title", title),
            ("description", description),
            ("access_token", access_token),
        ];
        let resp = self.http.post(format!("{}/{page_id}/videos", self.graph_url()))
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Delete a post by ID.
    pub async fn delete_post(
        &self, access_token: &str, post_id: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{post_id}", self.graph_url());
        let resp = self.http.delete(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("access_token", access_token)])
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Comment on a post.
    pub async fn comment_on_post(
        &self, access_token: &str, post_id: &str, message: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("message", message),
            ("access_token", access_token),
        ];
        let resp = self.http.post(format!("{}/{post_id}/comments", self.graph_url()))
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }    /// Get the actual playable video source URL for a video ID.
    /// Facebook's feed endpoint `media.source` often returns a thumbnail/preview.
    /// This makes a dedicated call to `/{video_id}?fields=source,embeddable,format`
    /// which returns the actual playable mp4 URL.
    pub async fn get_video_source(
        &self, access_token: &str, video_id: &str
    ) -> Result<String, ProviderError> {
        let url = format!(
            "{}/{video_id}?fields=source,embeddable,format",
            self.graph_url()
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            // Try direct `source` field first
            if let Some(source) = json["source"].as_str() {
                if !source.is_empty() {
                    return Ok(source.to_string());
                }
            }
            // Fallback: pick the highest-quality format from the `format` array
            if let Some(formats) = json["format"].as_array() {
                if let Some(best) = formats.iter()
                    .filter(|f| f["filetype"].as_str() == Some("mp4") && f["embeddable"].as_bool() == Some(true))
                    .max_by_key(|f| f["quality"].as_str().unwrap_or(""))
                {
                    if let Some(url_str) = best["embed_url"].as_str().or_else(|| best["preview_url"].as_str()) {
                        return Ok(url_str.to_string());
                    }
                }
                // If no embeddable mp4, try any mp4
                if let Some(best) = formats.iter()
                    .filter(|f| f["filetype"].as_str() == Some("mp4"))
                    .max_by_key(|f| f["quality"].as_str().unwrap_or(""))
                {
                    if let Some(url_str) = best["embed_url"].as_str().or_else(|| best["preview_url"].as_str()) {
                        return Ok(url_str.to_string());
                    }
                }
            }
            Err(ProviderError::Api("No usable video source found in response".into()))
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"]
                    .as_str().unwrap_or("Facebook API error")
                    .to_string()
            ))
        }
    }

    /// React to a post (LIKE, LOVE, WOW, HAHA, SAD, ANGRY).
    pub async fn react_to_post(
        &self, access_token: &str, post_id: &str, reaction_type: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("type", reaction_type),
            ("access_token", access_token),
        ];
        let resp = self.http.post(format!("{}/{post_id}/reactions", self.graph_url()))
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Get page insights for a given metric and period.
    pub async fn get_page_insights(
        &self, access_token: &str, page_id: &str, metric: &str, period: &str,
        since: Option<&str>, until: Option<&str>
    ) -> Result<serde_json::Value, ProviderError> {
        let mut url = format!(
            "{}/{page_id}/insights?metric={metric}&period={period}",
            self.graph_url()
        );
        if let Some(s) = since {
            url.push_str(&format!("&since={s}"));
        }
        if let Some(u) = until {
            url.push_str(&format!("&until={u}"));
        }
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Get page conversations (inbox).
    pub async fn get_page_conversations(
        &self, access_token: &str, page_id: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "{}/{page_id}/conversations?fields=id,snippet,updated_time,participants,message_count",
            self.graph_url()
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Get messages in a conversation.
    pub async fn get_conversation_messages(
        &self, access_token: &str, conversation_id: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "{}/{conversation_id}/messages?fields=id,message,from,created_time",
            self.graph_url()
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Send a message in a conversation.
    pub async fn send_message(
        &self, access_token: &str, conversation_id: &str, message: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("message", message),
            ("access_token", access_token),
        ];
        let resp = self.http.post(format!("{}/{conversation_id}/messages", self.graph_url()))
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Extract a single media attachment from a raw attachment JSON object.
    /// Checks `media.source` first (which returns the actual video/photo URL),
    /// falls back to `media.image.src` for thumbnails, then `url`.
    /// Determines mime type from URL extension when possible.
    fn extract_single_media(a: &serde_json::Value) -> Option<MediaAttachment> {
        let attach_type = a["type"].as_str().unwrap_or("");
        let is_video = attach_type.contains("video") || attach_type == "animated_video";

        // Check `media.source` first: this is the actual source URL for both videos and images.
        // Facebook returns `media.source` for video attachments and some image types.
        if let Some(url) = a["media"]["source"].as_str() {
            let mime = if is_video || url.contains(".mp4") || url.contains(".webm") || url.contains(".mov") {
                "video/mp4"
            } else if url.contains(".gif") {
                "image/gif"
            } else {
                "image/jpeg"
            };
            return Some(MediaAttachment {
                url: url.to_string(),
                mime_type: mime.into(),
                alt: None,
                poster_url: None,
            });
        }

        // Fallback to `media.image.src` (thumbnail / photo URL)
        if let Some(url) = a["media"]["image"]["src"].as_str() {
            return Some(MediaAttachment {
                url: url.to_string(),
                mime_type: "image/jpeg".into(),
                alt: None,
                poster_url: None,
            });
        }

        // Fallback: use the generic attachment URL
        if let Some(url) = a["url"].as_str() {
            let mime = if url.contains(".mp4") || url.contains(".webm") || url.contains(".mov") {
                "video/mp4"
            } else {
                "image/jpeg"
            };
            return Some(MediaAttachment {
                url: url.to_string(),
                mime_type: mime.into(),
                alt: None,
                poster_url: None,
            });
        }

        None
    }

    /// Search for Facebook pages by query string.
    pub async fn search_pages(
        &self, access_token: &str, query: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self.http
            .get(format!("{}/search", self.graph_url()))
            .query(&[("q", query), ("type", "page")])
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }

    /// Get albums for a page.
    pub async fn get_page_albums(
        &self, access_token: &str, page_id: &str
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "{}/{page_id}/albums?fields=id,name,count,cover_photo,created_time",
            self.graph_url()
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 429 {
            Err(ProviderError::RateLimited("Facebook API rate limit".into()))
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"].as_str().unwrap_or("Facebook API error").to_string()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            database_url: "test".into(),
            jwt_secret: "test".into(),
            app_password: "test".into(),
            app_url: "http://localhost:3000".into(),
            frontend_url: "http://localhost:4200".into(),
            x_client_id: Some("test".into()),
            x_client_secret: Some("test".into()),
            x_auth_token: None,
            x_ct0: None,
            linkedin_client_id: Some("test".into()),
            linkedin_client_secret: Some("test".into()),
            bluesky_handle: None,
            bluesky_app_password: None,
            facebook_client_id: Some("test".into()),
            facebook_client_secret: Some("test".into()),
            instagram_client_id: Some("test".into()),
            instagram_client_secret: Some("test".into()),
        threads_app_id: Some("test".into()),
        threads_app_secret: Some("test".into()),
            youtube_client_id: Some("test".into()),
            youtube_client_secret: Some("test".into()),
            reddit_client_id: Some("test".into()),
            reddit_client_secret: Some("test".into()),
            reddit_username: Some("test".into()),
            reddit_password: Some("test".into()),
            reddit_access_token: Some("test".into()),
            reddit_refresh_token: Some("test".into()),
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
            instagram_app_id: Some("test".into()),
            instagram_app_secret: Some("test".into()),
            whatsapp_store_dir: None,
            slack_client_id: None,
            slack_client_secret: None,
            mastodon_client_id: None,
            mastodon_client_secret: None,
            mastodon_instance_url: None,
            hashnode_api_key: None,
            github_token: None,
            twitch_client_id: None,
            twitch_client_secret: None,
            vk_client_id: None,
            vk_client_secret: None,
            whop_client_id: None,
            whop_client_secret: None,
            mewe_client_id: None,
            mewe_client_secret: None,
            moltbook_client_id: None,
            moltbook_client_secret: None,
            kick_client_id: None,
            kick_client_secret: None,
            neynar_api_key: None,
            nostr_private_key: None,
            token_encryption_key: None,
            media_dir: "./uploads".into(),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            stripe_price_free: None,
            stripe_price_pro_monthly: None,
            stripe_price_pro_annual: None,
            stripe_price_business_monthly: None,
            stripe_price_business_annual: None,
        }
    }

    #[test]
    fn test_scopes_contain_required() {
        let provider = FacebookProvider::new(&test_config());
        let scopes = provider.scopes();
        assert!(scopes.contains(&"pages_show_list".to_string()));
        assert!(scopes.contains(&"pages_manage_posts".to_string()));
        assert!(scopes.contains(&"public_profile".to_string()));
    }

    #[test]
    fn test_identifier_and_name() {
        let provider = FacebookProvider::new(&test_config());
        assert_eq!(provider.identifier(), "facebook");
        assert_eq!(provider.name(), "Facebook");
    }

    #[test]
    fn test_max_content_length() {
        let provider = FacebookProvider::new(&test_config());
        assert_eq!(provider.max_content_length(), 63206);
    }

    #[tokio::test]
    async fn test_generate_auth_url_contains_params() {
        let provider = FacebookProvider::new(&test_config());
        let result = provider.generate_auth_url("test_state", "test_verifier", "http://localhost:3000/callback").await;
        let url = result.unwrap().url;

        assert!(url.contains("client_id=test"), "should contain client_id");
        assert!(url.contains("redirect_uri="), "should contain redirect_uri");
        assert!(url.contains("state=test_state"), "should contain state");
        assert!(url.contains("scope="), "should contain scope");
        assert!(url.contains("response_type=code"), "should contain response_type");
        assert!(url.starts_with("https://www.facebook.com/v21.0/dialog/oauth"));
    }
}
