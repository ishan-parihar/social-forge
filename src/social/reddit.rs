use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct RedditProvider {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    http: reqwest::Client,
}

impl RedditProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("reddit").unwrap_or_default();
        let username = config.reddit_username().unwrap_or_default();
        let password = config.reddit_password().unwrap_or_default();
        let access_token = config.reddit_access_token();
        let refresh_token = config.reddit_refresh_token();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.5"),
        );
        headers.insert(
            reqwest::header::CACHE_CONTROL,
            reqwest::header::HeaderValue::from_static("no-cache"),
        );

        let http = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .default_headers(headers)
            .cookie_store(true)
            .gzip(true)
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client_id,
            client_secret,
            username,
            password,
            access_token,
            refresh_token,
            http,
        }
    }

    async fn password_grant(&self) -> Result<(String, u32), ProviderError> {
        let auth = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );

        let resp = self
            .http
            .post("https://www.reddit.com/api/v1/access_token")
            .header("Authorization", format!("Basic {auth}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .form(&[
                ("grant_type", "password"),
                ("username", self.username.as_str()),
                ("password", self.password.as_str()),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::Api(format!(
                "Reddit password grant error ({}): {}",
                status, body
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::Api(format!(
                "Failed to parse Reddit token response: {e}. Body: {body}"
            )))?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let expires_in = json["expires_in"].as_u64().unwrap_or(3600) as u32;

        Ok((access_token, expires_in))
    }

    /// Fetch user info from /api/v1/me given an access token
    /// Helper: authenticated GET to oauth.reddit.com
    async fn get_oauth(
        &self,
        token: &str,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value, ProviderError> {
        let mut all_params: Vec<(&str, &str)> = params.to_vec();
        all_params.push(("raw_json", "1"));

        let resp = self
            .http
            .get(&format!("https://oauth.reddit.com{endpoint}"))
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .query(&all_params)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!(
                "Reddit API error ({status}): {body}"
            )));
        }

        Ok(resp.json().await?)
    }

    /// Browse a subreddit (equivalent to reddit-cli browse command)
    pub async fn browse(
        &self,
        token: &str,
        subreddit: &str,
        sort: &str,
        limit: u32,
        time: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let clean = subreddit.trim_start_matches("r/").to_lowercase();
        let endpoint = format!("/r/{clean}/{sort}");
        let limit_s = limit.to_string();
        let mut params: Vec<(&str, &str)> = vec![("limit", &limit_s)];
        if sort == "top" || sort == "controversial" {
            params.push(("t", time));
        }
        self.get_oauth(token, &endpoint, &params).await
    }

    /// Search Reddit (equivalent to reddit-cli search command)
    pub async fn search(
        &self,
        token: &str,
        query: &str,
        subreddit: Option<&str>,
        sort: &str,
        limit: u32,
        time: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let endpoint = if let Some(sub) = subreddit {
            let clean = sub.trim_start_matches("r/").to_lowercase();
            format!("/r/{clean}/search")
        } else {
            "/search".to_string()
        };
        let limit_s = limit.to_string();
        let restrict_sr = if subreddit.is_some() { "true" } else { "false" };
        let params: Vec<(&str, &str)> = vec![
            ("q", query),
            ("sort", sort),
            ("limit", &limit_s),
            ("t", time),
            ("restrict_sr", restrict_sr),
        ];
        self.get_oauth(token, &endpoint, &params).await
    }

    /// Get post details with comments (equivalent to reddit-cli post command)
    /// Returns (post_listing, comments_listing) as a JSON array with 2 elements
    pub async fn post_detail(
        &self,
        token: &str,
        post_id: &str,
        depth: u32,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let pid = post_id.trim_start_matches("t3_");
        let info_id = format!("t3_{pid}");
        let info = self
            .get_oauth(
                token,
                "/api/info",
                &[("id", &info_id)],
            )
            .await?;
        let subreddit = info["data"]["children"][0]["data"]["subreddit"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Post not found".into()))?
            .to_string();

        let endpoint = format!("/r/{subreddit}/comments/{pid}");
        let limit_s = limit.to_string();
        let depth_s = depth.to_string();
        let params: Vec<(&str, &str)> = vec![
            ("limit", &limit_s),
            ("depth", &depth_s),
        ];
        self.get_oauth(token, &endpoint, &params).await
    }

    pub async fn get_comments(
        &self,
        token: &str,
        post_id: &str,
        sort: &str,
        depth: u32,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let pid = post_id.trim_start_matches("t3_");
        let info_id = format!("t3_{pid}");
        let info = self
            .get_oauth(token, "/api/info", &[("id", &info_id)])
            .await?;
        let subreddit = info["data"]["children"][0]["data"]["subreddit"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Post not found".into()))?
            .to_string();

        let endpoint = format!("/r/{subreddit}/comments/{pid}");
        let limit_s = limit.to_string();
        let depth_s = depth.to_string();
        let params: Vec<(&str, &str)> = vec![
            ("limit", &limit_s),
            ("depth", &depth_s),
            ("sort", sort),
        ];
        self.get_oauth(token, &endpoint, &params).await
    }

    /// Get user info + optional posts + comments (equivalent to reddit-cli user command)
    pub async fn user_info(
        &self,
        token: &str,
        username: &str,
        include_posts: bool,
        include_comments: bool,
    ) -> Result<serde_json::Value, ProviderError> {
        let clean = username.trim_start_matches("u/");
        let about = self
            .get_oauth(token, &format!("/user/{clean}/about"), &[])
            .await?;

        let mut result = serde_json::json!({
            "about": about,
        });

        if include_posts {
            let posts = self
                .get_oauth(token, &format!("/user/{clean}/submitted"), &[("limit", "25")])
                .await?;
            result["posts"] = posts;
        }

        if include_comments {
            let comments = self
                .get_oauth(token, &format!("/user/{clean}/comments"), &[("limit", "25")])
                .await?;
            result["comments"] = comments;
        }

        Ok(result)
    }

    /// Send a direct message (reddit-cli DM equivalent — POST /api/compose)
    pub async fn send_dm(
        &self,
        token: &str,
        to: &str,
        subject: &str,
        text: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post("https://oauth.reddit.com/api/compose")
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .form(&[
                ("api_type", "json"),
                ("to", to),
                ("subject", subject),
                ("text", text),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!(
                "Reddit DM API error ({status}): {body}"
            )));
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        // Check for Reddit API-level errors
        if let Some(errors) = json["json"]["errors"].as_array() {
            if !errors.is_empty() {
                let msg = errors
                    .first()
                    .and_then(|e| e.as_array())
                    .and_then(|a| a.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(ProviderError::Api(format!("Reddit DM error: {msg}")));
            }
        }
        Ok(json)
    }

    /// List inbox/notifications (messages, mentions, replies)
    pub async fn inbox(
        &self,
        token: &str,
        folder: &str,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let endpoint = format!("/message/{folder}");
        self.get_oauth(token, &endpoint, &[("limit", &limit.to_string())])
            .await
    }

    async fn fetch_me(&self, token: &str) -> Result<AuthToken, ProviderError> {
        let resp = self
            .http
            .get("https://oauth.reddit.com/api/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        tracing::debug!("Reddit /api/v1/me response ({status}): {body}");

        if !status.is_success() {
            return Err(ProviderError::Auth(format!(
                "Reddit access token rejected by /api/v1/me ({status}): {body}"
            )));
        }

        let user: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => {
                return Err(ProviderError::Api(format!(
                    "Failed to parse Reddit /api/v1/me response: {e}. Body: {body}"
                )));
            }
        };

        Ok(AuthToken {
            access_token: token.to_string(),
            refresh_token: None,
            expires_in: Some(86400),
            provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["name"].as_str().unwrap_or("").to_string(),
            username: user["name"].as_str().unwrap_or("").to_string(),
            picture: user["icon_img"]
                .as_str()
                .and_then(|s| s.split('?').next())
                .map(String::from),
        })
    }
}

#[async_trait]
impl SocialProvider for RedditProvider {
    fn identifier(&self) -> &'static str {
        "reddit"
    }

    fn name(&self) -> &'static str {
        "Reddit"
    }

    fn scopes(&self) -> Vec<String> {
        vec!["read".into(), "identity".into(), "submit".into()]
    }

    fn max_content_length(&self) -> usize {
        10000
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        Err(ProviderError::Auth(
            "Reddit uses password grant, not OAuth. Set REDDIT_USERNAME + REDDIT_PASSWORD in .env"
                .into(),
        ))
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // 1. Try pre-configured REDDIT_ACCESS_TOKEN (from env, same as reddit-cli)
        if let Some(token) = &self.access_token {
            let resp = self
                .http
                .get("https://oauth.reddit.com/api/v1/me")
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
                .send()
                .await?;

            if resp.status().is_success() {
                let user: serde_json::Value = resp.json().await.unwrap_or_default();
                if user["name"].as_str().map_or(true, |s| s.is_empty()) {
                    // valid JSON response but no user info — token expired/revoked
                } else {
                    return Ok(AuthToken {
                        access_token: token.clone(),
                        refresh_token: self.refresh_token.clone(),
                        expires_in: Some(86400),
                        provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
                        name: user["name"].as_str().unwrap_or("").to_string(),
                        username: user["name"].as_str().unwrap_or("").to_string(),
                        picture: user["icon_img"]
                            .as_str()
                            .and_then(|s| s.split('?').next())
                            .map(String::from),
                    });
                }
            }
        }

        // 2. Try refresh_token to get a new access token (obtained via auth code flow)
        if let Some(rt) = &self.refresh_token {
            let auth = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", self.client_id, self.client_secret),
            );
            let resp = self
                .http
                .post("https://www.reddit.com/api/v1/access_token")
                .header("Authorization", format!("Basic {auth}"))
                .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
                .form(&[("grant_type", "refresh_token"), ("refresh_token", rt)])
                .send()
                .await?;

            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                let json: serde_json::Value = match serde_json::from_str(&body) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Reddit refresh token JSON parse error: {e}: {body}");
                        return Err(ProviderError::Api(format!(
                            "Reddit refresh token parse error: {e}"
                        )));
                    }
                };
                if let Some(new_token) = json["access_token"].as_str() {
                    tracing::debug!("Reddit refreshed access token OK, calling /api/v1/me");
                    return self.fetch_me(new_token).await;
                } else {
                    tracing::warn!("Reddit refresh token response missing access_token: {body}");
                }
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::warn!("Reddit refresh token request failed ({status}): {body}");
            }
        }

        // 3. Fall back to password grant (works for "script" type apps)
        if !self.username.is_empty() && !self.password.is_empty() {
            let (access_token, _expires_in) = self.password_grant().await?;
            return self.fetch_me(&access_token).await;
        }

        Err(ProviderError::Auth(
            "Reddit not configured. Set REDDIT_ACCESS_TOKEN + REDDIT_REFRESH_TOKEN (from auth code flow) or REDDIT_USERNAME + REDDIT_PASSWORD (script app) in .env"
                .into(),
        ))
    }

    async fn refresh_token(&self, _old_refresh_token: &str) -> Result<AuthToken, ProviderError> {
        // Use pre-configured REDDIT_REFRESH_TOKEN if available (auth code flow)
        if let Some(rt) = &self.refresh_token {
            let auth = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", self.client_id, self.client_secret),
            );

            let resp = self
                .http
                .post("https://www.reddit.com/api/v1/access_token")
                .header("Authorization", format!("Basic {auth}"))
                .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
                .form(&[("grant_type", "refresh_token"), ("refresh_token", rt)])
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                if let Some(new_token) = json["access_token"].as_str() {
                    return Ok(AuthToken {
                        access_token: new_token.to_string(),
                        refresh_token: self.refresh_token.clone(),
                        expires_in: json["expires_in"].as_u64().map(|e| e as u32),
                        provider_user_id: String::new(),
                        name: String::new(),
                        username: String::new(),
                        picture: None,
                    });
                }
            }
        }

        // Fall back to password grant (script app)
        if !self.username.is_empty() && !self.password.is_empty() {
            let (access_token, expires_in) = self.password_grant().await?;
            return Ok(AuthToken {
                access_token,
                refresh_token: None,
                expires_in: Some(expires_in),
                provider_user_id: String::new(),
                name: String::new(),
                username: String::new(),
                picture: None,
            });
        }

        Err(ProviderError::Auth(
            "Reddit token expired and no refresh mechanism configured. Set REDDIT_REFRESH_TOKEN or REDDIT_USERNAME/REDDIT_PASSWORD in .env"
                .into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let subreddit = post.settings["subreddit"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidRequest("Missing subreddit in settings".into()))?
            .replace("/r/", "")
            .to_lowercase();

        let title = post.settings["title"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidRequest("Missing title in settings".into()))?;

        let kind = if !post.media.is_empty() {
            if post.media[0].url.contains(".mp4") {
                "video"
            } else {
                "image"
            }
        } else {
            "self"
        };

        let mut post_data: Vec<(&str, &str)> = vec![
            ("api_type", "json"),
            ("title", title),
            ("kind", kind),
            ("text", &post.content),
            ("sr", &subreddit),
        ];

        if !post.media.is_empty() {
            post_data.push(("url", post.media[0].url.as_str()));
        }

        if let Some(flair) = post.settings["flair"].as_str() {
            post_data.push(("flair_id", flair));
        }

        let resp = self
            .http
            .post("https://oauth.reddit.com/api/submit")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .form(&post_data)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::Api(format!(
                "Reddit submit API error ({}): {}",
                status, body
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::Api(format!(
                "Failed to parse Reddit response: {e}. Body: {body}"
            )))?;

        let data = &json["json"]["data"];
        let post_id = data["id"].as_str().unwrap_or("").to_string();
        let post_url = data["url"].as_str().unwrap_or("").to_string();

        if post_id.is_empty() {
            let err = json["json"]["errors"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|e| e.as_array())
                .and_then(|e| e.get(1))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(ProviderError::Api(err.to_string()));
        }

        Ok(PublishResult {
            platform_post_id: post_id,
            platform_post_url: Some(post_url),
            status: "published".into(),
        })
    }

    async fn comment(
        &self,
        access_token: &str,
        post_id: &str,
        _last_comment_id: Option<&str>,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let thing_id = if post_id.starts_with("t3_") || post_id.starts_with("t1_") {
            post_id.to_string()
        } else {
            format!("t3_{}", post_id)
        };

        let resp = self
            .http
            .post("https://oauth.reddit.com/api/comment")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .form(&[
                ("api_type", "json"),
                ("thing_id", &thing_id),
                ("text", &post.content),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::Api(format!(
                "Reddit comment API error ({}): {}",
                status, body
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::Api(format!(
                "Failed to parse Reddit comment response: {e}. Body: {body}"
            )))?;

        let comment_id = json["json"]["data"]["things"][0]["data"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if comment_id.is_empty() {
            let err = json["json"]["errors"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|e| e.as_array())
                .and_then(|e| e.get(1))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(ProviderError::Api(err.to_string()));
        }

        Ok(PublishResult {
            platform_post_id: comment_id,
            platform_post_url: None,
            status: "published".into(),
        })
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let resp = self
            .http
            .get("https://oauth.reddit.com/api/v1/me/karma")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let mut result = Vec::new();

        if let Some(data) = json["data"].as_array() {
            for entry in data {
                let subreddit = entry["sr"].as_str().unwrap_or("unknown");
                let link_karma = entry["link_karma"].as_i64().unwrap_or(0);
                let comment_karma = entry["comment_karma"].as_i64().unwrap_or(0);

                result.push(AnalyticsData {
                    label: format!("r/{subreddit}"),
                    data: vec![
                        AnalyticsDataPoint {
                            total: link_karma.to_string(),
                            date: "link_karma".into(),
                        },
                        AnalyticsDataPoint {
                            total: comment_karma.to_string(),
                            date: "comment_karma".into(),
                        },
                    ],
                    percentage_change: 0.0,
                });
            }
        }

        Ok(result)
    }

    async fn search_mention(
        &self,
        access_token: &str,
        query: &str,
    ) -> Result<Vec<MentionResult>, ProviderError> {
        let resp = self
            .http
            .get("https://oauth.reddit.com/message/inbox")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "postiz-rust:v0.1.0 (by /u/postiz_rust)")
            .query(&[("limit", "25")])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let query_lower = query.to_lowercase();
        let mut result = Vec::new();

        if let Some(children) = json["data"]["children"].as_array() {
            for child in children {
                let msg = &child["data"];
                let subject = msg["subject"].as_str().unwrap_or("").to_lowercase();
                let author = msg["author"].as_str().unwrap_or("");

                if !query.is_empty() && !subject.contains(&query_lower) {
                    continue;
                }

                result.push(MentionResult {
                    id: msg["id"].as_str().unwrap_or("").to_string(),
                    label: format!(
                        "u/{author}: {}",
                        msg["subject"].as_str().unwrap_or("")
                    ),
                    image: None,
                    do_not_cache: None,
                });
            }
        }

        Ok(result)
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Reddit does not support page management".into(),
        ))
    }
}
