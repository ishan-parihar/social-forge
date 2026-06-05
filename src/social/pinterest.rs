// ─── Pinterest Provider ───────────────────────────────────────
// Pinterest API v5 OAuth2. Supports boards, pins, and video pins.

use async_trait::async_trait;

use super::*;
use crate::config::Config;
use reqwest::StatusCode;
use serde_json::{json, Value};

pub struct PinterestProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl PinterestProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("pinterest").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    pub async fn get_user_account(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://api.pinterest.com/v5/user_account")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("Pinterest API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_board(
        &self,
        access_token: &str,
        board_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.pinterest.com/v5/boards/{board_id}");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("Pinterest API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_board_pins(
        &self,
        access_token: &str,
        board_id: &str,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.pinterest.com/v5/boards/{board_id}/pins");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("page_size", &limit.clamp(1, 100).to_string())])
            .send()
            .await?;

        let status = resp.status();
        let json = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("Pinterest API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_pin(
        &self,
        access_token: &str,
        pin_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.pinterest.com/v5/pins/{pin_id}");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("Pinterest API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_board_analytics(
        &self,
        access_token: &str,
        board_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.pinterest.com/v5/boards/{board_id}/analytics");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("start_date", start_date), ("end_date", end_date)])
            .send()
            .await?;

        let status = resp.status();
        let json = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("Pinterest API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_pin_analytics(
        &self,
        access_token: &str,
        pin_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.pinterest.com/v5/pins/{pin_id}/analytics");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("start_date", start_date), ("end_date", end_date)])
            .send()
            .await?;

        let status = resp.status();
        let json = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("Pinterest API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Search pins by keyword using Pinterest API v5
    pub async fn search_pins(&self, access_token: &str, query: &str, limit: Option<u32>) -> Result<Value, ProviderError> {
        let max = limit.unwrap_or(25).min(100);
        let encoded_query: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let url = format!(
            "https://api.pinterest.com/v5/pins/search?query={}&page_size={}",
            encoded_query, max
        );
        let response = self.http.get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(json!({"raw": text}));
        if status.is_success() { Ok(v) }
        else if status == StatusCode::UNAUTHORIZED { Err(ProviderError::TokenExpired) }
        else if status == StatusCode::FORBIDDEN { Err(ProviderError::Auth(v["message"].as_str().unwrap_or("forbidden").into())) }
        else { Err(ProviderError::Api(v["message"].as_str().unwrap_or(&text).into())) }
    }
}

#[async_trait]
impl SocialProvider for PinterestProvider {
    fn identifier(&self) -> &'static str {
        "pinterest"
    }

    fn name(&self) -> &'static str {
        "Pinterest"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "boards:read".into(),
            "boards:write".into(),
            "pins:read".into(),
            "pins:write".into(),
            "user_accounts:read".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        500
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let scope = "boards:read,boards:write,pins:read,pins:write,user_accounts:read";
        let params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", scope),
            ("state", state),
        ];

        let url = url::Url::parse_with_params(
            "https://www.pinterest.com/oauth/",
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
        let auth_bytes = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );

        let params: Vec<(&str, &str)> = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .http
            .post("https://api.pinterest.com/v5/oauth/token")
            .header("Authorization", format!("Basic {auth_bytes}"))
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
            .get("https://api.pinterest.com/v5/user_account")
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
            name: user["username"].as_str().unwrap_or("").to_string(),
            username: user["username"].as_str().unwrap_or("").to_string(),
            picture: user["profile_image"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let auth_bytes = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );

        let scope = self.scopes().join(",");
        let params: Vec<(&str, &str)> = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", scope.as_str()),
        ];

        let resp = self
            .http
            .post("https://api.pinterest.com/v5/oauth/token")
            .header("Authorization", format!("Basic {auth_bytes}"))
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

    /// List boards for the authenticated user
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get("https://api.pinterest.com/v5/boards")
            .query(&[("page_size", "250")])
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let items = json["items"].as_array().cloned().unwrap_or_default();

        Ok(items
            .iter()
            .map(|item| PageInfo {
                id: item["id"].as_str().unwrap_or("").to_string(),
                name: item["name"].as_str().unwrap_or("").to_string(),
                access_token: None,
                picture: None,
                username: None,
            })
            .collect())
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Pinterest does not support page-level management".into(),
        ))
    }

    async fn get_recent_posts(&self, access_token: &str, _internal_id: &str, limit: u32) -> Result<Vec<ExternalPostData>, ProviderError> {
        let boards = self.pages(access_token).await?;
        let mut posts = Vec::new();
        let max_pins = (limit / 5).max(1).min(10);

        for board in boards.iter().take(max_pins as usize) {
            let pins = self.get_board_pins(access_token, &board.id, limit.min(50)).await?;
            if let Some(items) = pins["items"].as_array() {
                for item in items {
                    let pin_id = item["id"].as_str().unwrap_or("").to_string();
                    let description = item["description"].as_str().unwrap_or("").to_string();
                    let title = item["title"].as_str().map(|s| s.to_string());
                    let media_url = item["media"]["images"]["originals"]["url"]
                        .as_str()
                        .map(|s| s.to_string());
                    let link = item["link"].as_str().map(|s| s.to_string());
                    let created_at = item["created_at"].as_str().unwrap_or("");

                    let posted_at = crate::social::common::parse_timestamp(created_at);

                    let post_url = link.or_else(|| {
                        Some(format!("https://www.pinterest.com/pin/{}", item["id"].as_str().unwrap_or("")))
                    });

                    posts.push(ExternalPostData {
                        platform_post_id: pin_id,
                        text: description,
                        author_name: None,
                        author_handle: None,
                        author_avatar: None,
                        media: media_url.into_iter().map(|u| MediaAttachment {
                            url: u,
                            mime_type: String::new(),
                            alt: None,
                        }).collect(),
                        created_at: posted_at,
                        url: post_url,
                        metadata: title.map(|t| serde_json::json!({"title": t})),
                    });
                }
            }
        }

        // Sort by posted_at descending, take top 20
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        posts.truncate(20);

        Ok(posts)
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let board_id = post.settings["board"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidRequest("Missing board in settings".into()))?;

        let title = post.settings["title"].as_str().unwrap_or("");
        let link = post.settings["link"].as_str().unwrap_or("");

        // Construct media source
        let media_source = if !post.media.is_empty() && post.media[0].url.contains(".mp4") {
            // Video: upload first via /media endpoint, then create pin
            return self.publish_video(access_token, post, board_id).await;
        } else if post.media.len() == 1 {
            serde_json::json!({
                "source_type": "image_url",
                "url": post.media[0].url
            })
        } else {
            let items: Vec<serde_json::Value> = post
                .media
                .iter()
                .map(|m| serde_json::json!({"url": m.url}))
                .collect();
            serde_json::json!({
                "source_type": "multiple_image_urls",
                "items": items
            })
        };

        let mut body = serde_json::json!({
            "board_id": board_id,
            "description": post.content,
            "media_source": media_source,
        });

        if !title.is_empty() {
            body["title"] = serde_json::json!(title);
        }
        if !link.is_empty() {
            body["link"] = serde_json::json!(link);
        }

        let resp = self
            .http
            .post("https://api.pinterest.com/v5/pins")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        let pin_id = json["id"]
            .as_str()
            .ok_or_else(|| {
                let err = json["message"]
                    .as_str()
                    .unwrap_or("Pinterest publish failed");
                ProviderError::Api(err.to_string())
            })?;

        Ok(PublishResult {
            platform_post_id: pin_id.to_string(),
            platform_post_url: Some(format!("https://www.pinterest.com/pin/{pin_id}")),
            status: "published".into(),
        })
    }
}

impl PinterestProvider {
    async fn publish_video(
        &self,
        access_token: &str,
        _post: &PostContent,
        _board_id: &str,
    ) -> Result<PublishResult, ProviderError> {
        // Step 1: Register media upload
        let reg_resp = self
            .http
            .post("https://api.pinterest.com/v5/media")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&serde_json::json!({"media_type": "video"}))
            .send()
            .await?;

        let _reg_json: serde_json::Value = reg_resp.json().await?;
        // Note: actual video upload requires presigned URL handling
        // Stub: return error with guidance
        Err(ProviderError::Api(
            "Video pin upload requires additional setup. Use image pins instead.".into(),
        ))
    }
}
