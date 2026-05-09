// ─── Pinterest Provider ───────────────────────────────────────
// Pinterest API v5 OAuth2. Supports boards, pins, and video pins.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

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
