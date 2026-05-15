// ─── Threads Provider ─────────────────────────────────────────
// Meta Threads API v1.0 via graph.threads.net.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct ThreadsProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl ThreadsProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("threads").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn graph_url(&self) -> &'static str {
        "https://graph.threads.net/v1.0"
    }
}

#[async_trait]
impl SocialProvider for ThreadsProvider {
    fn identifier(&self) -> &'static str {
        "threads"
    }

    fn name(&self) -> &'static str {
        "Threads"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "threads_basic".into(),
            "threads_content_publish".into(),
            "threads_delete".into(),
            "threads_keyword_search".into(),
            "threads_manage_insights".into(),
            "threads_manage_mentions".into(),
            "threads_manage_replies".into(),
            "threads_profile_discovery".into(),
            "threads_read_replies".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        500
    }

    fn needs_cron_refresh(&self) -> bool {
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
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("state", state),
            ("scope", scope.as_str()),
        ];

        let url = url::Url::parse_with_params(
            "https://www.threads.net/oauth/authorize",
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
        // Step 1: Exchange code for short-lived token
        let token_params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
        ];

        let resp = self
            .http
            .get("https://graph.threads.net/oauth/access_token")
            .query(&token_params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let err_msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            let err_code = json["error"]["code"].as_u64().unwrap_or(0);
            return Err(ProviderError::Api(format!(
                "Threads token exchange failed (code {err_code}): {err_msg}"
            )));
        }
        let short_token = json["access_token"]
            .as_str()
            .ok_or_else(|| {
                let err = serde_json::to_string(&json).unwrap_or_default();
                ProviderError::Auth(format!("Missing access_token in response: {err}"))
            })?
            .to_string();

        // Step 2: Exchange for long-lived token (60 days)
        let long_params: Vec<(&str, &str)> = vec![
            ("grant_type", "th_exchange_token"),
            ("client_secret", self.client_secret.as_str()),
            ("access_token", short_token.as_str()),
        ];

        let long_resp = self
            .http
            .get("https://graph.threads.net/access_token")
            .query(&long_params)
            .send()
            .await?;

        let long_status = long_resp.status();
        let long_json: serde_json::Value = long_resp.json().await?;
        if !long_status.is_success() {
            let err_msg = long_json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            return Err(ProviderError::Api(format!(
                "Threads long-lived token exchange failed: {err_msg}"
            )));
        }
        let access_token = long_json["access_token"]
            .as_str()
            .unwrap_or(&short_token)
            .to_string();
        let expires_in = long_json["expires_in"].as_u64().map(|v| v as u32);

        // Get user info
        let user: serde_json::Value = self
            .http
            .get(format!("{}/me", self.graph_url()))
            .query(&[
                ("fields", "id,username,threads_profile_picture_url"),
                ("access_token", &access_token),
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
            provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["username"].as_str().unwrap_or("").to_string(),
            username: user["username"].as_str().unwrap_or("").to_string(),
            picture: user["threads_profile_picture_url"]
                .as_str()
                .map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("grant_type", "th_refresh_token"),
            ("access_token", refresh_token),
        ];

        let resp = self
            .http
            .get("https://graph.threads.net/refresh_access_token")
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
            .get(format!("{}/me", self.graph_url()))
            .query(&[
                ("fields", "id,username,threads_profile_picture_url"),
                ("access_token", &access_token),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(AuthToken {
            access_token: access_token.clone(),
            refresh_token: Some(access_token),
            expires_in,
            provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["username"].as_str().unwrap_or("").to_string(),
            username: user["username"].as_str().unwrap_or("").to_string(),
            picture: user["threads_profile_picture_url"]
                .as_str()
                .map(String::from),
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let user_id = self.resolve_user_id(access_token).await?;

        // Build media params based on content type
        let (media_type, url_key): (&str, &str) = if post.media.is_empty() {
            ("TEXT", "")
        } else if post.media.len() == 1 && post.media[0].url.contains(".mp4") {
            ("VIDEO", "video_url")
        } else if post.media.len() == 1 {
            ("IMAGE", "image_url")
        } else {
            // Carousel: create child containers first
            return self.publish_carousel(&user_id, access_token, post).await;
        };

        // Create container
        let mut form: Vec<(&str, &str)> = vec![
            ("media_type", media_type),
            ("text", &post.content),
            ("access_token", access_token),
        ];

        if !url_key.is_empty() {
            form.push((url_key, post.media[0].url.as_str()));
        }

        let resp = self
            .http
            .post(format!("{}/{user_id}/threads", self.graph_url()))
            .form(&form)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let creation_id = json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Api(format!("No creation ID: {json:?}")))?
            .to_string();

        // Publish
        let pub_resp = self
            .http
            .post(format!(
                "{}/{user_id}/threads_publish",
                self.graph_url()
            ))
            .form(&[("creation_id", creation_id.as_str()), ("access_token", access_token)])
            .send()
            .await?;

        let pub_json: serde_json::Value = pub_resp.json().await?;
        let thread_id = pub_json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Api(format!("Publish failed: {pub_json:?}")))?
            .to_string();

        let permalink = pub_json["permalink"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(PublishResult {
            platform_post_id: thread_id,
            platform_post_url: Some(permalink),
            status: "published".into(),
        })
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Threads does not support page management".into(),
        ))
    }

    fn map_error(&self, body: &str, _status: u16) -> Option<String> {
        if body.contains("Error validating access token") {
            Some("Threads access token expired".into())
        } else if body.contains("text must be at most 500 characters") {
            Some("Post text exceeds 500 characters limit".into())
        } else {
            None
        }
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let user_id = self.resolve_user_id(access_token).await?;

        let since = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string();
        let until = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let json = self
            .get_insights(
                access_token,
                &user_id,
                "views,likes,replies,reposts,quotes",
                "day",
            )
            .await?;

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
        let resp = self
            .http
            .get(format!("{}/{platform_post_id}/insights", self.graph_url()))
            .query(&[
                ("metric", "views,likes,replies,reposts,quotes"),
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

impl ThreadsProvider {
    async fn resolve_user_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let user: serde_json::Value = self
            .http
            .get(format!("{}/me", self.graph_url()))
            .query(&[("fields", "id"), ("access_token", access_token)])
            .send()
            .await?
            .json()
            .await?;

        user["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ProviderError::Auth("Could not resolve Threads user ID".into()))
    }

    pub async fn get_profile(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/me", self.graph_url()))
            .query(&[
                ("fields", "id,username,name,threads_profile_picture_url"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("get_profile failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn get_threads(
        &self,
        access_token: &str,
        user_id: &str,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let effective_limit = if limit < 100 { 100 } else { limit };
        let resp = self
            .http
            .get(format!("{}/{user_id}/threads", self.graph_url()))
            .query(&[
                ("fields", "id,text,media_type,media_url,permalink,timestamp,like_count,reply_count"),
                ("access_token", access_token),
            ])
            .query(&[("limit", effective_limit.to_string())])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("get_threads failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn get_thread_detail(
        &self,
        access_token: &str,
        media_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/{}", self.graph_url(), media_id))
            .query(&[
                ("fields", "id,text,media_type,media_url,permalink,timestamp,username,like_count,reply_count,children{id,media_url,media_type}"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("get_thread_detail failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn get_thread_replies(
        &self,
        access_token: &str,
        media_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/{}/replies", self.graph_url(), media_id))
            .query(&[
                ("fields", "id,text,timestamp,username,like_count"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("get_thread_replies failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn reply_to_thread(
        &self,
        access_token: &str,
        media_id: &str,
        message: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post(format!("{}/{}/replies", self.graph_url(), media_id))
            .form(&[("text", message), ("access_token", access_token)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("reply_to_thread failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn get_insights(
        &self,
        access_token: &str,
        user_id: &str,
        metric: &str,
        period: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/{}/threads_insights", self.graph_url(), user_id))
            .query(&[("metric", metric), ("period", period), ("access_token", access_token)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("get_insights failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn delete_thread(
        &self,
        access_token: &str,
        media_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!("{}/{}", self.graph_url(), media_id))
            .query(&[("access_token", access_token)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(ProviderError::Api(format!("delete_thread failed: {}", body)));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    async fn publish_carousel(
        &self,
        user_id: &str,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Create child containers
        let mut child_ids = Vec::new();
        for media in &post.media {
            let is_video = media.url.contains(".mp4");
            let media_type = if is_video { "VIDEO" } else { "IMAGE" };
            let url_key = if is_video { "video_url" } else { "image_url" };

            let resp = self
                .http
                .post(format!("{}/{user_id}/threads", self.graph_url()))
                .form(&[
                    ("media_type", media_type),
                    (url_key, media.url.as_str()),
                    ("is_carousel_item", "true"),
                    ("access_token", access_token),
                ])
                .send()
                .await?;

            let json: serde_json::Value = resp.json().await?;
            let child_id = json["id"]
                .as_str()
                .ok_or_else(|| ProviderError::Api(format!("Carousel child fail: {json:?}")))?
                .to_string();
            child_ids.push(child_id);
        }

        // Create CAROUSEL container
        let children_csv = child_ids.join(",");
        let car_resp = self
            .http
            .post(format!("{}/{user_id}/threads", self.graph_url()))
            .form(&[
                ("media_type", "CAROUSEL"),
                ("text", &post.content),
                ("children", &children_csv),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let car_json: serde_json::Value = car_resp.json().await?;
        let creation_id = car_json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Api(format!("Carousel fail: {car_json:?}")))?
            .to_string();

        // Publish
        let pub_resp = self
            .http
            .post(format!(
                "{}/{user_id}/threads_publish",
                self.graph_url()
            ))
            .form(&[("creation_id", creation_id.as_str()), ("access_token", access_token)])
            .send()
            .await?;

        let pub_json: serde_json::Value = pub_resp.json().await?;

        Ok(PublishResult {
            platform_post_id: pub_json["id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            platform_post_url: pub_json["permalink"]
                .as_str()
                .map(String::from),
            status: "published".into(),
        })
    }
}
