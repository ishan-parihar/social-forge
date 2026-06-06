// ─── VK Provider ───────────────────────────────────────────
// Uses VK OAuth 2.0 + VK API for wall posts.
// Supports: OAuth flow, user info, wall posting, comments.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct VkProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl VkProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("vk").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    const API_VERSION: &'static str = "5.199";

    /// Make a VK API call with access_token passed as a parameter.
    async fn api_call(
        &self,
        method: &str,
        access_token: &str,
        mut params: Vec<(&str, &str)>,
    ) -> Result<serde_json::Value, ProviderError> {
        params.push(("access_token", access_token));
        params.push(("v", Self::API_VERSION));

        let resp = self
            .http
            .post(format!("https://api.vk.com/method/{method}"))
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        // Check for VK API-level errors
        if let Some(error) = json["error"].as_object() {
            let error_code = error["error_code"].as_i64().unwrap_or(0);
            let error_msg = error["error_msg"]
                .as_str()
                .unwrap_or("Unknown VK API error")
                .to_string();

            return match error_code {
                5 | 15 => Err(ProviderError::TokenExpired),
                _ => Err(ProviderError::Api(format!("VK error {error_code}: {error_msg}"))),
            };
        }

        if !status.is_success() {
            return Err(ProviderError::Api(format!("VK API HTTP error ({status})")));
        }

        Ok(json)
    }
}

#[async_trait]
impl SocialProvider for VkProvider {
    fn identifier(&self) -> &'static str {
        "vk"
    }

    fn name(&self) -> &'static str {
        "VK"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "wall".into(),
            "groups".into(),
            "photos".into(),
            "video".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        15000
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Normal
    }

    fn uses_oauth(&self) -> bool {
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
            ("display", "page"),
            ("redirect_uri", redirect_uri),
            ("scope", scope.as_str()),
            ("response_type", "code"),
            ("state", state),
        ];

        let url = url::Url::parse_with_params(
            "https://oauth.vk.com/authorize",
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
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .http
            .post("https://oauth.vk.com/access_token")
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["error_description"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("Token exchange failed")
                .to_string();
            return Err(ProviderError::Auth(msg));
        }

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);
        let user_id = json["user_id"].as_i64().unwrap_or(0).to_string();
        let email = json["email"].as_str().map(String::from);

        // Fetch user info
        let user_info = self
            .api_call(
                "users.get",
                &access_token,
                vec![("user_ids", &user_id), ("fields", "photo_max_orig")],
            )
            .await?;

        let user = user_info["response"][0].clone();
        let first_name = user["first_name"].as_str().unwrap_or("").to_string();
        let last_name = user["last_name"].as_str().unwrap_or("").to_string();
        let full_name = if first_name.is_empty() && last_name.is_empty() {
            email.clone().unwrap_or_default()
        } else {
            format!("{first_name} {last_name}")
        };

        Ok(AuthToken {
            access_token,
            refresh_token: None, // VK short-term tokens don't easily refresh
            expires_in,
            provider_user_id: user_id,
            name: full_name,
            username: email.unwrap_or_default(),
            picture: user["photo_max_orig"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, _token: &str) -> Result<AuthToken, ProviderError> {
        // VK OAuth tokens (for client-side flow) are typically short-lived
        // and don't support refresh tokens easily. Users re-authenticate.
        Err(ProviderError::Auth(
            "VK token refresh not supported. Please re-authenticate.".into(),
        ))
    }

    /// Publish a post to the user's wall.
    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let json = self
            .api_call(
                "wall.post",
                access_token,
                vec![("message", &post.content)],
            )
            .await?;

        let post_id_val = json["response"]["post_id"].as_i64().unwrap_or(0);
        if post_id_val == 0 {
            return Err(ProviderError::Api("Missing post_id in VK API response".into()));
        }
        let post_id = post_id_val.to_string();

        Ok(PublishResult {
            platform_post_id: post_id.clone(),
            platform_post_url: Some(format!("https://vk.com/wall{post_id}")),
            status: "published".into(),
        })
    }

    async fn get_recent_posts(&self, access_token: &str, internal_id: &str, limit: u32) -> Result<Vec<ExternalPostData>, ProviderError> {
        let owner_id = if internal_id.is_empty() { "" } else { internal_id };
        let count = limit.min(100).to_string();
        let count_ref: &str = &count;
        let mut params = vec![("count", count_ref)];
        if !owner_id.is_empty() {
            params.push(("owner_id", owner_id));
        }

        let json = self.api_call("wall.get", access_token, params).await?;
        let mut posts = Vec::new();

        if let Some(items) = json["response"]["items"].as_array() {
            for item in items {
                let post_id = item["id"].as_i64().unwrap_or(0).to_string();
                let text_val = item["text"].as_str().unwrap_or("").to_string();
                let date_ts = item["date"].as_i64().unwrap_or(0);

                // Extract media attachments
                let mut media = Vec::new();
                if let Some(attachments) = item["attachments"].as_array() {
                    for att in attachments {
                        if let Some(photo) = att["photo"].as_object() {
                            if let Some(sizes) = photo["sizes"].as_array() {
                                if let Some(largest) = sizes.last() {
                                    if let Some(url) = largest["url"].as_str() {
                                        media.push(url.to_string());
                                    }
                                }
                            }
                        }
                    }
                }

                let posted_at = chrono::DateTime::from_timestamp(date_ts, 0)
                    .unwrap_or_default();

                let post_url = Some(format!("https://vk.com/wall{owner_id}_{post_id}"));
                posts.push(ExternalPostData {
                    platform_post_id: post_id,
                    text: text_val,
                    author_name: None,
                    author_handle: None,
                    author_avatar: None,
                    media: media.into_iter().map(|u| MediaAttachment {
                        url: u,
                        mime_type: String::new(),
                        alt: None,
                        poster_url: None,
                    }).collect(),
                    created_at: posted_at,
                    url: post_url,
                    metadata: None,
                });
            }
        }

        Ok(posts)
    }

    /// Return the authenticated user as a single page.
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let user_info = self
            .api_call(
                "users.get",
                access_token,
                vec![("fields", "photo_max_orig")],
            )
            .await
            .unwrap_or_default();

        let user = user_info["response"][0].clone();
        let first_name = user["first_name"].as_str().unwrap_or("").to_string();
        let last_name = user["last_name"].as_str().unwrap_or("").to_string();
        let full_name = if first_name.is_empty() && last_name.is_empty() {
            "VK User".to_string()
        } else {
            format!("{first_name} {last_name}")
        };

        Ok(vec![PageInfo {
            id: user["id"].as_i64().unwrap_or(0).to_string(),
            name: full_name,
            access_token: Some(access_token.to_string()),
            picture: user["photo_max_orig"].as_str().map(String::from),
            username: None,
        }])
    }

    /// Fetch page info by user ID.
    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let user_info = self
            .api_call(
                "users.get",
                access_token,
                vec![("user_ids", page_id), ("fields", "photo_max_orig")],
            )
            .await
            .unwrap_or_default();

        let user = user_info["response"][0].clone();
        let first_name = user["first_name"].as_str().unwrap_or("").to_string();
        let last_name = user["last_name"].as_str().unwrap_or("").to_string();

        Ok(PageInfo {
            id: user["id"].as_i64().unwrap_or(0).to_string(),
            name: format!("{first_name} {last_name}"),
            access_token: Some(access_token.to_string()),
            picture: user["photo_max_orig"].as_str().map(String::from),
            username: None,
        })
    }

    /// Comment on a wall post.
    async fn comment(
        &self,
        access_token: &str,
        post_id: &str,
        _last_comment_id: Option<&str>,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let json = self
            .api_call(
                "wall.createComment",
                access_token,
                vec![("post_id", post_id), ("message", &post.content)],
            )
            .await?;

        let comment_id = json["response"]["comment_id"]
            .as_i64()
            .unwrap_or(0)
            .to_string();

        Ok(PublishResult {
            platform_post_id: comment_id,
            platform_post_url: None,
            status: "published".into(),
        })
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("VK token expired. Re-authenticate via VK OAuth.".into())
        } else {
            None
        }
    }
}
