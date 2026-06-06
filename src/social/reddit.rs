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
    /// Cookie string for www.reddit.com auth (dual-path)
    cookie_string: Option<String>,
    /// Cached modhash (CSRF token) for cookie-based POST requests
    modhash: Option<String>,
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
            reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.5"),
        );
        headers.insert(
            reqwest::header::CACHE_CONTROL,
            reqwest::header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            reqwest::header::PRAGMA,
            reqwest::header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            reqwest::header::UPGRADE_INSECURE_REQUESTS,
            reqwest::header::HeaderValue::from_static("1"),
        );

        let http = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .default_headers(headers)
            .http1_only()
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
            cookie_string: None,
            modhash: None,
        }
    }

    // ── Cookie Auth (dual-path) ─────────────────────────────────

    /// Initialize from a token JSON blob (cookie or OAuth).
    pub fn prepare_from_token(&mut self, token: &str) {
        if let Some((session, token_v2, cookie_str)) = crate::social::reddit_cookies::parse_cookie_token(token) {
            self.set_cookie_credentials(&session, token_v2.as_deref(), cookie_str.as_deref());
        }
    }

    /// Check if a token is a cookie auth JSON blob
    pub fn is_cookie_auth(token: &str) -> bool {
        crate::social::reddit_cookies::is_cookie_auth(token)
    }

    fn set_cookie_credentials(&mut self, reddit_session: &str, token_v2: Option<&str>, extra_cookies: Option<&str>) {
        self.cookie_string = Some(match extra_cookies {
            Some(extras) if !extras.is_empty() && extras.contains("reddit_session=") => {
                extras.to_string()
            }
            _ => {
                let mut s = format!("reddit_session={reddit_session}");
                if let Some(t) = token_v2 {
                    s.push_str(&format!("; token_v2={t}"));
                }
                s
            }
        });
    }

    fn has_cookies(&self) -> bool {
        self.cookie_string.is_some()
    }

    /// GET request via www.reddit.com with cookie auth
    pub async fn get_www(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value, ProviderError> {
        let cookies = self.cookie_string.as_deref()
            .ok_or_else(|| ProviderError::Auth("No Reddit cookies configured".into()))?;

        let mut all_params: Vec<(&str, &str)> = params.to_vec();
        all_params.push(("raw_json", "1"));

        let resp = self.http
            .get(&format!("https://www.reddit.com{endpoint}"))
            .header("Cookie", cookies)
            .query(&all_params)
            .send()
            .await?;

        let status = resp.status();
        if status == 401 || status == 403 {
            return Err(ProviderError::TokenExpired);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("Reddit www API error ({status}): {body}")));
        }
        Ok(resp.json().await?)
    }

    /// POST request via www.reddit.com with cookie auth + modhash
    async fn post_www(
        &mut self,
        endpoint: &str,
        form: &[(&str, &str)],
    ) -> Result<serde_json::Value, ProviderError> {
        let modhash = self.ensure_modhash().await?;
        let cookies = self.cookie_string.as_deref()
            .ok_or_else(|| ProviderError::Auth("No Reddit cookies configured".into()))?
            .to_string();

        let mut all_form: Vec<(&str, &str)> = form.to_vec();
        all_form.push(("uh", &modhash));
        all_form.push(("api_type", "json"));

        let resp = self.http
            .post(&format!("https://www.reddit.com{endpoint}"))
            .header("Cookie", &cookies)
            .form(&all_form)
            .send()
            .await?;

        let status = resp.status();
        if status == 401 || status == 403 {
            return Err(ProviderError::TokenExpired);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("Reddit www POST error ({status}): {body}")));
        }
        Ok(resp.json().await?)
    }

    /// Fetch modhash (CSRF token) from /api/me.json
    async fn ensure_modhash(&mut self) -> Result<String, ProviderError> {
        if let Some(ref mh) = self.modhash {
            return Ok(mh.clone());
        }
        let me = self.get_www("/api/me.json", &[]).await?;
        let mh = me["data"]["modhash"].as_str().unwrap_or("").to_string();
        if mh.is_empty() {
            return Err(ProviderError::Auth("Could not fetch modhash — cookies may be expired".into()));
        }
        self.modhash = Some(mh.clone());
        Ok(mh)
    }

    // ── Cookie-based write operations ───────────────────────────

    /// Vote on a post/comment (dir: 1=upvote, 0=unvote, -1=downvote)
    pub async fn vote(&mut self, thing_id: &str, dir: i8) -> Result<serde_json::Value, ProviderError> {
        let dir_s = dir.to_string();
        let id = thing_id.to_string();
        self.post_www("/api/vote", &[("id", &id), ("dir", &dir_s)]).await
    }

    /// Save a post/comment
    pub async fn save(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/save", &[("id", &id)]).await
    }

    /// Unsave a post/comment
    pub async fn unsave(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/unsave", &[("id", &id)]).await
    }

    /// Hide a post
    pub async fn hide(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/hide", &[("id", &id)]).await
    }

    /// Unhide a post
    pub async fn unhide(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/unhide", &[("id", &id)]).await
    }

    /// Subscribe/unsubscribe to a subreddit (action: "sub" or "unsub")
    pub async fn subscribe(&mut self, subreddit: &str, action: &str) -> Result<serde_json::Value, ProviderError> {
        let sr = subreddit.trim_start_matches("r/").to_string();
        let act = action.to_string();
        self.post_www("/api/subscribe", &[("sr_name", &sr), ("action", &act)]).await
    }

    /// Edit a self-post or comment text
    pub async fn edit_text(&mut self, thing_id: &str, text: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        let txt = text.to_string();
        self.post_www("/api/editusertext", &[("thing_id", &id), ("text", &txt)]).await
    }

    /// Delete a post or comment
    pub async fn delete(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/del", &[("id", &id)]).await
    }

    /// Moderation: remove a post/comment (spam=true marks as spam)
    pub async fn mod_remove(&mut self, thing_id: &str, spam: bool) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        let spam_s = spam.to_string();
        self.post_www("/api/remove", &[("id", &id), ("spam", &spam_s)]).await
    }

    /// Moderation: approve a post/comment
    pub async fn mod_approve(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/approve", &[("id", &id)]).await
    }

    /// Moderation: distinguish a comment (how: "yes", "no", "admin", "special")
    pub async fn mod_distinguish(&mut self, thing_id: &str, how: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        let h = how.to_string();
        self.post_www("/api/distinguish", &[("id", &id), ("how", &h)]).await
    }

    /// Moderation: sticky/unsticky a post
    pub async fn mod_sticky(&mut self, thing_id: &str, state: bool) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        let s = state.to_string();
        self.post_www("/api/set_subreddit_sticky", &[("id", &id), ("state", &s)]).await
    }

    /// Moderation: lock a post/comment
    pub async fn mod_lock(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/lock", &[("id", &id)]).await
    }

    /// Moderation: unlock a post/comment
    pub async fn mod_unlock(&mut self, thing_id: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        self.post_www("/api/unlock", &[("id", &id)]).await
    }

    /// Submit a post via cookie auth (www.reddit.com)
    pub async fn submit_www(
        &mut self,
        subreddit: &str,
        title: &str,
        kind: &str,
        text: Option<&str>,
        url: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let sr = subreddit.trim_start_matches("r/").to_string();
        let t = title.to_string();
        let k = kind.to_string();
        let txt = text.unwrap_or("").to_string();
        let u = url.unwrap_or("").to_string();

        let mut form: Vec<(&str, &str)> = vec![
            ("sr", &sr),
            ("title", &t),
            ("kind", &k),
        ];
        if kind == "self" { form.push(("text", &txt)); }
        else { form.push(("url", &u)); }

        self.post_www("/api/submit", &form).await
    }

    /// Comment via cookie auth (www.reddit.com)
    pub async fn comment_www(&mut self, thing_id: &str, text: &str) -> Result<serde_json::Value, ProviderError> {
        let id = thing_id.to_string();
        let txt = text.to_string();
        self.post_www("/api/comment", &[("thing_id", &id), ("text", &txt)]).await
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
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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

    /// Parse a Reddit listing JSON response into ExternalPostData vec
    fn parse_reddit_listing(response: serde_json::Value) -> Result<Vec<ExternalPostData>, ProviderError> {
        let children = response["data"]["children"].as_array()
            .map(|c| c.to_vec())
            .unwrap_or_default();

        let mut posts = Vec::new();
        for child in &children {
            let data = &child["data"];
            let id = data["id"].as_str().unwrap_or("").to_string();
            if id.is_empty() { continue; }

            let title = data["title"].as_str().unwrap_or("").to_string();
            let selftext = data["selftext"].as_str().unwrap_or("").to_string();
            let text = if selftext.is_empty() { title.clone() } else { format!("{title}\n\n{selftext}") };

            let created_utc = data["created_utc"].as_f64().unwrap_or(0.0);
            let created_at = chrono::DateTime::from_timestamp(created_utc as i64, 0)
                .unwrap_or_else(chrono::Utc::now);

            let permalink = data["permalink"].as_str().unwrap_or("").to_string();
            let url = if permalink.is_empty() {
                None
            } else {
                Some(format!("https://www.reddit.com{permalink}"))
            };

            // Extract author info
            let author = data["author"].as_str().map(String::from);
            let avatar = data["author_icon_img"].as_str()
                .or_else(|| data["icon_img"].as_str())
                .and_then(|s| s.split('?').next())
                .map(String::from);

            let mut media = Vec::new();

            // Handle gallery posts (multi-image) — parse media_metadata + gallery_data
            let is_gallery = data["is_gallery"].as_bool().unwrap_or(false);
            if is_gallery {
                if let Some(metadata) = data["media_metadata"].as_object() {
                    // Use gallery_data for ordering; fall back to iterating metadata keys
                    let ordered_ids: Vec<&str> = if let Some(items) = data["gallery_data"]["items"].as_array() {
                        items.iter()
                            .filter_map(|item| item["media_id"].as_str())
                            .collect()
                    } else {
                        metadata.keys().map(|k| k.as_str()).collect()
                    };
                    for media_id in ordered_ids {
                        if let Some(meta) = metadata.get(media_id) {
                            // Use full-resolution image from `s.u`
                            let url = meta["s"]["u"].as_str()
                                .or_else(|| meta["s"]["gif"].as_str())
                                .or_else(|| {
                                    // Fallback: use the largest preview image
                                    meta["p"].as_array()
                                        .and_then(|p| p.last())
                                        .and_then(|last| last["u"].as_str())
                                });
                            if let Some(url) = url {
                                let mime = meta["m"].as_str().unwrap_or("image/jpeg");
                                media.push(MediaAttachment {
                                    url: url.to_string(),
                                    mime_type: mime.to_string(),
                                    alt: None,
                                    poster_url: None,
                                });
                            }
                        }
                    }
                }
            }

            // For non-gallery posts, use the regular media extraction
            if media.is_empty() {
                // Check for hosted:video (v.redd.it video posts)
                if let Some(hosted_video) = data["media"]["reddit_video"].as_object() {
                    if let Some(video_url) = hosted_video["fallback_url"].as_str() {
                        media.push(MediaAttachment {
                            url: video_url.to_string(),
                            mime_type: "video/mp4".to_string(),
                            alt: None,
                            poster_url: None,
                        });
                    }
                }

                // Check for standard URL (images, external links)
                if media.is_empty() {
                    if let Some(url_str) = data["url"].as_str() {
                        if !url_str.is_empty() && !selftext.contains(url_str) {
                            let is_video = url_str.contains("v.redd.it")
                                && (url_str.ends_with(".mp4") || url_str.contains("DASH_"));
                            let is_image = url_str.ends_with(".jpg")
                                || url_str.ends_with(".jpeg")
                                || url_str.ends_with(".png")
                                || url_str.ends_with(".gif")
                                || url_str.ends_with(".webp");
                            let mime_type = if is_video {
                                "video/mp4".to_string()
                            } else if url_str.contains(".gif") {
                                "image/gif".to_string()
                            } else if is_image {
                                "image/jpeg".to_string()
                            } else {
                                "image/jpeg".to_string()
                            };
                            media.push(MediaAttachment {
                                url: url_str.to_string(),
                                mime_type,
                                alt: None,
                                poster_url: None,
                            });
                        }
                    }
                }

                // Fallback: use preview image if no media found yet
                if media.is_empty() {
                    if let Some(preview) = data["preview"]["images"].as_array() {
                        if let Some(img) = preview.first() {
                            if let Some(source) = img["source"]["url"].as_str() {
                                // Decode Reddit's HTML-encoded URLs
                                let decoded = source.replace("&amp;", "&");
                                media.push(MediaAttachment {
                                    url: decoded,
                                    mime_type: "image/jpeg".to_string(),
                                    alt: None,
                                    poster_url: None,
                                });
                            }
                        }
                    }
                }
            }

            posts.push(ExternalPostData {
                platform_post_id: id,
                text,
                url,
                media,
                created_at,
                author_name: author.clone(),
                author_avatar: avatar,
                author_handle: author,
                metadata: None,
            });
        }

        Ok(posts)
    }

    /// Fetch recent posts via cookie-based www.reddit.com API
    /// Makes inline HTTP requests since self.cookie_string isn't set on the Arc-wrapped provider.
    async fn get_recent_posts_via_cookies(
        &self,
        access_token: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        let (_session, _token_v2, extra_cookies) = crate::social::reddit_cookies::parse_cookie_token(access_token)
            .ok_or_else(|| ProviderError::Auth("Invalid Reddit cookie token".into()))?;

        let cookie_str = extra_cookies.as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ProviderError::Auth(
                "Reddit cookie token missing cookie_string — re-authenticate via Reddit cookies".into()
            ))?;

        // Helper closure: cookie-authenticated GET to www.reddit.com
        async fn cookie_get(
            http: &reqwest::Client,
            cookie: &str,
            url: &str,
            params: &[(&str, &str)],
        ) -> Result<serde_json::Value, ProviderError> {
            let resp = http
                .get(url)
                .header("Cookie", cookie)
                .query(params)
                .send()
                .await
                .map_err(ProviderError::Network)?;
            let status = resp.status();
            if status == 401 || status == 403 {
                return Err(ProviderError::TokenExpired);
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ProviderError::Api(format!("Reddit www API error ({status}): {body}")));
            }
            resp.json().await.map_err(ProviderError::Network)
        }

        // Fetch authenticated user info via www.reddit.com with cookie auth
        let me = cookie_get(
            &self.http,
            cookie_str,
            "https://www.reddit.com/api/me.json",
            &[("raw_json", "1")],
        ).await?;

        let username = me["data"]["name"].as_str()
            .ok_or_else(|| ProviderError::Api("Could not determine Reddit username from cookies".into()))?;

        let limit_str = limit.to_string();
        let response = cookie_get(
            &self.http,
            cookie_str,
            &format!("https://www.reddit.com/user/{username}/submitted.json"),
            &[("limit", &limit_str), ("raw_json", "1")],
        ).await?;

        Self::parse_reddit_listing(response)
    }

    async fn fetch_me(&self, token: &str) -> Result<AuthToken, ProviderError> {
        let resp = self
            .http
            .get("https://oauth.reddit.com/api/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        if self.client_id.is_empty() || self.client_secret.is_empty() {
            return Err(ProviderError::Auth(
                "Reddit OAuth requires REDDIT_CLIENT_ID + REDDIT_CLIENT_SECRET".into(),
            ));
        }
        let scopes = self.scopes().join(" ");
        let url = format!(
            "https://www.reddit.com/api/v1/authorize?client_id={}&response_type=code&state={}&redirect_uri={}&duration=permanent&scope={}",
            self.client_id,
            state,
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
        );
        Ok(AuthUrlResponse { url })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // 0. If an authorization code is provided, exchange it (OAuth code flow)
        if !code.is_empty() && !self.client_id.is_empty() {
            let auth = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", self.client_id, self.client_secret),
            );
            let resp = self
                .http
                .post("https://www.reddit.com/api/v1/access_token")
                .header("Authorization", format!("Basic {auth}"))
                .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                ])
                .send()
                .await?;

            let body = resp.text().await.unwrap_or_default();
            let json: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| ProviderError::Api(format!("Reddit token exchange parse error: {e}")))?;

            if let Some(access_token) = json["access_token"].as_str() {
                let refresh_token = json["refresh_token"].as_str().map(String::from);
                let expires_in = json["expires_in"].as_u64().map(|n| n as u32);
                // Fetch user info
                let me_resp = self
                    .http
                    .get("https://oauth.reddit.com/api/v1/me")
                    .header("Authorization", format!("Bearer {access_token}"))
                    .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
                    .send()
                    .await?;
                let user: serde_json::Value = me_resp.json().await.unwrap_or_default();
                return Ok(AuthToken {
                    access_token: access_token.to_string(),
                    refresh_token,
                    expires_in: expires_in.or(Some(86400)),
                    provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
                    name: user["name"].as_str().unwrap_or("").to_string(),
                    username: user["name"].as_str().unwrap_or("").to_string(),
                    picture: user["icon_img"]
                        .as_str()
                        .and_then(|s| s.split('?').next())
                        .map(String::from),
                });
            } else {
                let error = json["error"].as_str().unwrap_or("unknown");
                return Err(ProviderError::Api(format!("Reddit code exchange failed: {error}")));
            }
        }

        // 1. Try pre-configured REDDIT_ACCESS_TOKEN (from env, same as reddit-cli)
        if let Some(token) = &self.access_token {
            let resp = self
                .http
                .get("https://oauth.reddit.com/api/v1/me")
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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
                .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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
                .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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

        if let Some(flair_id) = post.settings["flair_id"].as_str() {
            post_data.push(("flair_id", flair_id));
        } else if let Some(flair_text) = post.settings["flair_text"].as_str() {
            post_data.push(("flair_text", flair_text));
        }

        let resp = self
            .http
            .post("https://oauth.reddit.com/api/submit")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let fullname = if platform_post_id.starts_with("t3_") {
            platform_post_id.to_string()
        } else {
            format!("t3_{}", platform_post_id)
        };

        let resp = self
            .http
            .get(format!("https://oauth.reddit.com/by_id/{}", fullname))
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let data = &json["data"]["children"][0]["data"];

        let score = data["score"].as_i64().unwrap_or(0);
        let upvote_ratio = data["upvote_ratio"].as_f64().unwrap_or(0.0);
        let num_comments = data["num_comments"].as_i64().unwrap_or(0);
        let gilded = data["gilded"].as_i64().unwrap_or(0);
        let total_awards = data["total_awards_received"].as_i64().unwrap_or(0);

        let mut result = Vec::new();
        result.push(AnalyticsData {
            label: "score".into(),
            data: vec![AnalyticsDataPoint {
                total: score.to_string(),
                date: String::new(),
            }],
            percentage_change: 0.0,
        });
        result.push(AnalyticsData {
            label: "upvote_ratio".into(),
            data: vec![AnalyticsDataPoint {
                total: format!("{:.2}", upvote_ratio),
                date: String::new(),
            }],
            percentage_change: 0.0,
        });
        result.push(AnalyticsData {
            label: "num_comments".into(),
            data: vec![AnalyticsDataPoint {
                total: num_comments.to_string(),
                date: String::new(),
            }],
            percentage_change: 0.0,
        });
        result.push(AnalyticsData {
            label: "gilded".into(),
            data: vec![AnalyticsDataPoint {
                total: gilded.to_string(),
                date: String::new(),
            }],
            percentage_change: 0.0,
        });
        result.push(AnalyticsData {
            label: "total_awards_received".into(),
            data: vec![AnalyticsDataPoint {
                total: total_awards.to_string(),
                date: String::new(),
            }],
            percentage_change: 0.0,
        });

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
            .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
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

    async fn targets(&self, access_token: &str) -> Result<Vec<TargetInfo>, ProviderError> {
        let response = match self.get_oauth(access_token, "/subreddits/mine/subscriber", &[("limit", "100")]).await {
            Ok(r) => r,
            Err(_) => return Ok(vec![]),
        };

        let children = response["data"]["children"]
            .as_array()
            .map(|c| c.to_vec())
            .unwrap_or_default();

        let targets = children
            .iter()
            .filter_map(|child| {
                let data = &child["data"];
                let display_name = data["display_name"].as_str()?;
                let icon_img = data["icon_img"].as_str();
                let picture = icon_img.and_then(|s| s.split('?').next()).map(String::from);

                let metadata = serde_json::json!({
                    "subscribers": data["subscribers"],
                    "description": data["public_description"],
                    "over18": data["over18"],
                });

                Some(TargetInfo {
                    id: display_name.to_string(),
                    name: format!("r/{display_name}"),
                    target_type: "subreddit".to_string(),
                    picture,
                    metadata: Some(metadata),
                })
            })
            .collect();

        Ok(targets)
    }

    async fn get_recent_posts(
        &self,
        access_token: &str,
        _internal_id: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        // Handle cookie-based auth (users who authenticated via Reddit cookies)
        if Self::is_cookie_auth(access_token) {
            return self.get_recent_posts_via_cookies(access_token, limit).await;
        }

        // OAuth path: use Bearer token with oauth.reddit.com
        let me = self.get_oauth(access_token, "/api/v1/me", &[]).await?;
        let username = me["name"].as_str()
            .ok_or_else(|| ProviderError::Api("Could not determine Reddit username".into()))?;

        tracing::info!("Reddit get_recent_posts: username='{}' limit={}", username, limit);

        let limit_str = limit.to_string();
        let response = self.get_oauth(
            access_token,
            &format!("/user/{username}/submitted"),
            &[("limit", &limit_str)],
        ).await?;

        let children_count = response["data"]["children"].as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!(
            "Reddit get_recent_posts: {} children returned",
            children_count,
        );

        Self::parse_reddit_listing(response)
    }



    async fn get_post_engagement(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        let pid = platform_post_id.trim_start_matches("t3_");
        let info_id = format!("t3_{pid}");
        let info = self.get_oauth(access_token, "/api/info", &[("id", &info_id)]).await?;
        let child = info["data"]["children"][0]["data"].clone();
        if child.is_null() {
            return Ok(None);
        }
        Ok(Some(serde_json::json!({
            "score": child["score"],
            "num_comments": child["num_comments"],
            "upvote_ratio": child["upvote_ratio"],
            "downs": child["downs"],
            "ups": child["ups"],
        })))
    }
}
