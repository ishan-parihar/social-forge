// ─── Instagram Provider ───────────────────────────────────────
// OAuth 2.0 + Instagram Graph API (for Business/Creator accounts).
// Posts to Instagram Business accounts via Facebook's Graph API.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct InstagramProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl InstagramProvider {
    pub fn new(config: &Config) -> Self {
        // Instagram Graph API uses Facebook Login OAuth — needs Facebook App credentials
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
impl SocialProvider for InstagramProvider {
    fn identifier(&self) -> &'static str {
        "instagram"
    }

    fn name(&self) -> &'static str {
        "Instagram"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "instagram_basic".into(),
            "instagram_content_publish".into(),
            "instagram_manage_comments".into(),
            "instagram_manage_insights".into(),
            "instagram_manage_messages".into(),
            "pages_show_list".into(),
            "pages_read_engagement".into(),
            "business_management".into(),
            "pages_manage_engagement".into(),
            "pages_manage_metadata".into(),
            "pages_read_user_content".into(),
            "read_insights".into(),
            "pages_messaging".into(),
        ]
    }

    fn is_between_steps(&self) -> bool { true }

    fn max_content_length(&self) -> usize {
        2200
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
        // Exchange code for token
        let token_params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("code", code),
        ];

        let resp = self
            .http
            .get("https://graph.facebook.com/v21.0/oauth/access_token")
            .query(&token_params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let short_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Get long-lived token
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

        // Get Facebook user info for display
        let me: serde_json::Value = self
            .http
            .get(format!("{}/me?fields=id,name,picture.type(large)", self.graph_url()))
            .query(&[("access_token", &access_token)])
            .send()
            .await?
            .json()
            .await?;

        let fb_id = me["id"].as_str().unwrap_or("").to_string();
        let fb_name = me["name"].as_str().unwrap_or("Instagram").to_string();
        let fb_picture = me["picture"]["data"]["url"].as_str().map(String::from);

        // Return user-level token so pages() + page-picker can discover IG accounts
        Ok(AuthToken {
            access_token,             // user-level token for page discovery
            refresh_token: None,      // page token will be set when user picks an account
            expires_in,
            provider_user_id: fb_id,
            name: fb_name,
            username: String::new(),
            picture: fb_picture,
        })
    }

    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Instagram tokens last 60 days. Reconnect the channel.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        if post.media.is_empty() {
            return Err(ProviderError::InvalidRequest(
                "Instagram Feed posts require at least one media attachment (image or video).".into()
            ));
        }

        let ig_id = self.resolve_ig_business_account(access_token).await
            .map_err(|e| ProviderError::Api(format!("Cannot resolve IG business account: {e}")))?;

        if post.media.len() == 1 {
            // Single image: simpler IMAGE container
            let container_id = self.create_single_media_container(
                &ig_id, access_token, &post.media[0].url, &post.content, false,
            ).await?;

            let platform_post_id = self.publish_container(&ig_id, access_token, &container_id).await?;
            Ok(PublishResult {
                platform_post_url: Some(format!("https://instagram.com/p/{platform_post_id}")),
                platform_post_id,
                status: "published".into(),
            })
        } else {
            // Multiple images: CAROUSEL flow
            let mut children_ids = Vec::with_capacity(post.media.len());
            for media in &post.media {
                let child_id = self.create_single_media_container(
                    &ig_id, access_token, &media.url, "", true,
                ).await?;
                children_ids.push(child_id);
            }

            // Create CAROUSEL container with children
            let children_str = children_ids.join(",");
            let carousel_resp = self
                .http
                .post(format!("{}/{}/media", self.graph_url(), ig_id))
                .form(&[
                    ("media_type", "CAROUSEL"),
                    ("children", &children_str),
                    ("caption", &post.content),
                    ("access_token", access_token),
                ])
                .send()
                .await?;

            let carousel_json: serde_json::Value = carousel_resp.json().await?;

            if let Some(err) = carousel_json["error"].as_object() {
                let msg = err["message"].as_str().unwrap_or("Carousel creation failed");
                return Err(ProviderError::Api(msg.to_string()));
            }

            let carousel_id = carousel_json["id"]
                .as_str()
                .ok_or_else(|| ProviderError::Api(
                    format!("Instagram did not return carousel container ID: {carousel_json:?}")
                ))?;

            let platform_post_id = self.publish_container(&ig_id, access_token, carousel_id).await?;
            Ok(PublishResult {
                platform_post_url: Some(format!("https://instagram.com/p/{platform_post_id}")),
                platform_post_id,
                status: "published".into(),
            })
        }
    }

    /// List Instagram Business accounts from the user's Facebook pages.
    ///
    /// Discovers FB pages via two phases:
    ///   Phase 1 - `/me/accounts` — pages the user selected in the OAuth dialog
    ///   Phase 2 - `/me/businesses` → `owned_pages` + `client_pages` — pages from Business Manager
    ///
    /// Filters to only pages with a linked Instagram Business account (either via the
    /// `instagram_business_account` field or the `/{page_id}/instagram_business_account`
    /// endpoint) and resolves IG account details.
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let fields = "id,instagram_business_account,access_token,username,name,picture.type(large)";
        let mut pages_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();

        // ── Helper: paginated fetch ──────────────────────────────────────────
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
                    // Non-fatal: skip this endpoint (e.g. /me/businesses may not be available)
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

        // ── Phase 1: /me/accounts ────────────────────────────────────────────
        let me_url = format!(
            "{}/me/accounts?fields={}&limit=100",
            self.graph_url(),
            fields
        );
        collect_pages(&self.http, &me_url, access_token, &mut pages_map).await?;

        // ── Phase 2: /me/businesses → owned_pages + client_pages ─────────────
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

        // ── Resolve IG Business accounts from collected pages ─────────────────
        let mut result = Vec::new();

        for (_page_id, page) in &pages_map {
            let page_token = page["access_token"].as_str().unwrap_or(access_token);

            // Try to get the IG Business account ID (prefer the inline field)
            let ig_id = page["instagram_business_account"]["id"]
                .as_str()
                .map(String::from);

            let ig_id = if let Some(id) = ig_id {
                Some(id)
            } else {
                // Fallback: call /{page_id}/instagram_business_account
                let ig_resp = self
                    .http
                    .get(format!(
                        "{}/{}/instagram_business_account",
                        self.graph_url(),
                        page["id"].as_str().unwrap_or("")
                    ))
                    .query(&[("access_token", page_token)])
                    .send()
                    .await?;

                let ig: serde_json::Value = ig_resp.json().await?;
                if ig["error"].is_null() {
                    ig["id"].as_str().map(String::from)
                } else {
                    continue; // No IG account for this page
                }
            };

            if let Some(ig_id) = ig_id {
                // Resolve IG account details
                let ig_resp = self
                    .http
                    .get(format!(
                        "{}/{}?fields=id,username,name,profile_picture_url",
                        self.graph_url(),
                        ig_id
                    ))
                    .query(&[("access_token", page_token)])
                    .send()
                    .await?;

                let ig_json: serde_json::Value = ig_resp.json().await?;

                if let Some(e) = ig_json["error"].as_object() {
                    tracing::warn!("IG detail fetch for {ig_id}: {}", e["message"].as_str().unwrap_or(""));
                    continue;
                }

                let name = ig_json["username"]
                    .as_str()
                    .or_else(|| ig_json["name"].as_str())
                    .unwrap_or("")
                    .to_string();

                let profile_pic = ig_json["profile_picture_url"].as_str().map(String::from);

                result.push(PageInfo {
                    id: ig_id,
                    name,
                    access_token: Some(page_token.to_string()),
                    picture: profile_pic,
                    username: None,
                });
            }
        }

        Ok(result)
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/{page_id}?fields=id,username,name,profile_picture_url", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        Ok(PageInfo {
            id: json["id"].as_str().unwrap_or("").to_string(),
            name: json["name"].as_str().unwrap_or("").to_string(),
            access_token: None,
            picture: json["profile_picture_url"].as_str().map(String::from),
            username: json["username"].as_str().map(String::from),
        })
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let ig_id = self
            .resolve_ig_business_account(access_token)
            .await
            .map_err(|e| ProviderError::Api(format!("Failed to resolve IG business account: {e}")))?;

        let since = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string();
        let until = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let url = format!("{}/{ig_id}/insights", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("metric", "impressions,reach,profile_views,follower_count"),
                ("period", "day"),
                ("since", since.as_str()),
                ("until", until.as_str()),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
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
                ("metric", "engagement,impressions,reach,saved"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
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
}

impl InstagramProvider {
    /// Create a single IMAGE container and return its container ID.
    async fn create_single_media_container(
        &self,
        ig_id: &str,
        access_token: &str,
        media_url: &str,
        caption: &str,
        is_carousel_item: bool,
    ) -> Result<String, ProviderError> {
        let mut params = vec![
            ("image_url", media_url),
            ("access_token", access_token),
        ];
        if is_carousel_item {
            params.push(("is_carousel_item", "true"));
        } else {
            params.push(("caption", caption));
        }

        let resp = self
            .http
            .post(format!("{}/{}/media", self.graph_url(), ig_id))
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        if let Some(err) = json["error"].as_object() {
            let msg = err["message"].as_str().unwrap_or("Container creation failed");
            return Err(ProviderError::Api(msg.to_string()));
        }

        json["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                ProviderError::Api(format!("Instagram did not return container ID: {json:?}"))
            })
    }

    /// Publish a container (single image or carousel) and return the IG media ID.
    async fn publish_container(
        &self,
        ig_id: &str,
        access_token: &str,
        creation_id: &str,
    ) -> Result<String, ProviderError> {
        let resp = self
            .http
            .post(format!("{}/{}/media_publish", self.graph_url(), ig_id))
            .form(&[
                ("creation_id", creation_id),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        if let Some(err) = json["error"].as_object() {
            let msg = err["message"].as_str().unwrap_or("Publish failed");
            return Err(ProviderError::Api(msg.to_string()));
        }

        json["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                ProviderError::Api(format!("Instagram publish unexpected response: {json:?}"))
            })
    }

    /// Resolve the Instagram Business Account ID from a page-scoped token.
    async fn resolve_ig_business_account(&self, access_token: &str) -> Result<String, String> {
        // Get the user's Facebook pages
        let resp = self
            .http
            .get(format!("{}/me/accounts", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await
            .map_err(|e| format!("Failed to get Facebook pages: {e}"))?;

        let accounts: serde_json::Value = resp.json().await
            .map_err(|e| format!("Failed to parse pages response: {e}"))?;

        if let Some(err) = accounts["error"].as_object() {
            return Err(format!(
                "Facebook API error: {}",
                err["message"].as_str().unwrap_or("unknown")
            ));
        }

        let pages = accounts["data"].as_array()
            .ok_or_else(|| "No Facebook pages found for this token.".to_string())?;

        for page in pages {
            let page_id = page["id"].as_str().unwrap_or("");
            let page_token = page["access_token"].as_str().unwrap_or(access_token);

            let ig: serde_json::Value = self
                .http
                .get(format!("{}/{page_id}/instagram_business_account", self.graph_url()))
                .query(&[("access_token", page_token)])
                .send()
                .await
                .map_err(|e| format!("Failed to check IG for page {page_id}: {e}"))?
                .json()
                .await
                .map_err(|e| format!("Failed to parse IG response: {e}"))?;

            if let Some(ig_obj) = ig.as_object() {
                if let Some(ig_id) = ig_obj.get("id").and_then(|v| v.as_str()) {
                    // The IG Business Account API returns the ID at the top level
                    return Ok(ig_id.to_string());
                }
            }
        }

        Err("No Instagram Business account found. Link Instagram to a Facebook Page first.".to_string())
    }
}

impl InstagramProvider {
    pub async fn get_ig_media(&self, access_token: &str, ig_id: &str, limit: u32) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.min(100);
        let url = format!("{}/{ig_id}/media", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,caption,media_type,media_url,permalink,timestamp,like_count,comments_count"), ("limit", &limit.to_string())])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_media_detail(&self, access_token: &str, media_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{media_id}", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,caption,media_type,media_url,permalink,timestamp,username,like_count,comments_count,children{id,media_url,media_type}")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_media_comments(&self, access_token: &str, media_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{media_id}/comments", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,text,timestamp,username,like_count,replies")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn search_hashtag(&self, access_token: &str, ig_id: &str, query: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/ig_hashtag_search", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("user_id", ig_id), ("q", query)])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_hashtag_media(&self, access_token: &str, hashtag_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{hashtag_id}/recent_media", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,caption,media_type,media_url,permalink,timestamp,like_count,comments_count")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_insights(&self, access_token: &str, ig_id: &str, metric: &str, period: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/insights", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("metric", metric), ("period", period)])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_tagged(&self, access_token: &str, ig_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/tagged", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,caption,media_type,media_url,permalink,timestamp")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn create_ig_container(&self, access_token: &str, ig_id: &str, media_type: &str, media_url: &str, caption: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/media", self.graph_url());
        let mut params = vec![
            ("media_type", media_type),
            ("caption", caption),
            ("access_token", access_token),
        ];
        if media_type == "IMAGE" {
            params.push(("image_url", media_url));
        } else {
            params.push(("media_url", media_url));
        }
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&params)
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn publish_ig_container(&self, access_token: &str, ig_id: &str, creation_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/media_publish", self.graph_url());
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn reply_to_ig_comment(&self, access_token: &str, comment_id: &str, message: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{comment_id}/replies", self.graph_url());
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_reels(&self, access_token: &str, ig_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/media", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,caption,media_type,media_url,permalink,timestamp"), ("media_type", "REELS")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_stories(&self, access_token: &str, ig_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/stories", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,media_type,media_url,permalink,timestamp")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_followers(&self, access_token: &str, ig_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "name,username,followers_count,follows_count,media_count,profile_picture_url")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_business_discovery(&self, access_token: &str, ig_id: &str, target_username: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/business_discovery", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("username", target_username), ("fields", "biography,followers_count,follows_count,media_count,profile_picture_url,username,name,media{id,caption,media_type,media_url,permalink,timestamp,like_count,comments_count}")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_mentions(&self, access_token: &str, ig_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/mentions", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,caption,media_type,media_url,permalink,timestamp")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn get_ig_insights_audience(&self, access_token: &str, ig_id: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/{ig_id}/insights", self.graph_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("metric", "audience_city,audience_country,audience_gender,audience_age_range"), ("period", "lifetime")])
            .header("Authorization", format!("Bearer {access_token}"))
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
            let detail = json["error"]["message"].as_str().unwrap_or("Instagram API error").to_string();
            Err(ProviderError::Api(detail))
        }
    }

    pub async fn poll_container_status(
        &self,
        access_token: &str,
        creation_id: &str,
    ) -> Result<String, ProviderError> {
        let url = format!("{}/{}", self.graph_url(), creation_id);
        let resp = self
            .http
            .get(&url)
            .query(&[("fields", "id,status_code")])
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        if let Some(err) = json["error"].as_object() {
            let msg = err["message"].as_str().unwrap_or("Container status check failed");
            return Err(ProviderError::Api(msg.to_string()));
        }
        let status_code = json["status_code"]
            .as_str()
            .unwrap_or("IN_PROGRESS");
        Ok(status_code.to_string())
    }
}
