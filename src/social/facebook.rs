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
        ]
    }

    fn max_content_length(&self) -> usize {
        63206
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

    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/{page_id}?fields=id,name,picture.type(large)", self.graph_url()))
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
            "{}/{page_id}/feed?fields=message,created_time,story,attachments&limit={limit}",
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

    /// Get comments on a post.
    pub async fn get_post_comments(
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
            app_url: "http://localhost:3000".into(),
            frontend_url: "http://localhost:4200".into(),
            x_client_id: Some("test".into()),
            x_client_secret: Some("test".into()),
            linkedin_client_id: Some("test".into()),
            linkedin_client_secret: Some("test".into()),
            bluesky_handle: None,
            bluesky_app_password: None,
            facebook_client_id: Some("test".into()),
            facebook_client_secret: Some("test".into()),
            instagram_client_id: Some("test".into()),
            instagram_client_secret: Some("test".into()),
            threads_client_id: Some("test".into()),
            threads_client_secret: Some("test".into()),
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
            telegram_token: Some("test".into()),
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
            token_encryption_key: None,
            media_dir: "./uploads".into(),
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
