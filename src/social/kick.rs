// ─── Kick Provider ────────────────────────────────────────────
// Uses Kick OAuth 2.0 + Kick API for channel announcements.
// Supports: OAuth flow, announcements, user info.

use async_trait::async_trait;

use super::*;
use crate::config::Config;
use tracing;

const KICK_API_BASE: &str = "https://api.kick.com";

pub struct KickProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl KickProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("kick").unwrap_or_default();
        if client_id.is_empty() || client_secret.is_empty() {
            tracing::warn!("Kick provider initialized with empty credentials");
        }
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    /// Fetch the authenticated user's info from Kick API.
    async fn get_users(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{KICK_API_BASE}/api/v2/users"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]
                .as_str()
                .or_else(|| json["message"].as_str())
                .unwrap_or("Unknown Kick API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }
}

#[async_trait]
impl SocialProvider for KickProvider {
    fn identifier(&self) -> &'static str {
        "kick"
    }

    fn name(&self) -> &'static str {
        "Kick"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "user:read".into(),
            "channel:post".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        500
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
        let scope = self.scopes().join(" ");
        let params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("scope", scope.as_str()),
            ("redirect_uri", redirect_uri),
            ("state", state),
            ("response_type", "code"),
        ];

        let url = url::Url::parse_with_params(
            "https://id.kick.com/oauth/authorize",
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
        let params = serde_json::json!({
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "code": code,
            "grant_type": "authorization_code",
            "redirect_uri": redirect_uri,
        });

        let resp = self
            .http
            .post("https://id.kick.com/oauth/token")
            .header("Content-Type", "application/json")
            .json(&params)
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
        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        // Fetch user info from Kick API
        let user_info = self.get_users(&access_token).await?;
        let user_data = user_info["data"].as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or_default();
        let provider_user_id = user_data["id"].as_str().unwrap_or("").to_string();
        let name = user_data["name"]
            .as_str()
            .or_else(|| user_data["display_name"].as_str())
            .unwrap_or("")
            .to_string();
        let username = user_data["username"].as_str().unwrap_or("").to_string();
        let picture = user_data["profile_image"]
            .as_str()
            .or_else(|| user_data["avatar"].as_str())
            .map(String::from);

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id,
            name,
            username,
            picture,
        })
    }

    async fn refresh_token(&self, token: &str) -> Result<AuthToken, ProviderError> {
        let params = serde_json::json!({
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "grant_type": "refresh_token",
            "refresh_token": token,
        });

        let resp = self
            .http
            .post("https://id.kick.com/oauth/token")
            .header("Content-Type", "application/json")
            .json(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["error_description"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("Token refresh failed")
                .to_string();
            return Err(ProviderError::Auth(msg));
        }

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let new_refresh = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        Ok(AuthToken {
            access_token,
            refresh_token: new_refresh.or_else(|| Some(token.to_string())),
            expires_in,
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    /// Publish a channel announcement on Kick.
    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let channel_id = post
            .settings
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let body = serde_json::json!({
            "content": post.content,
            "channel_id": channel_id,
        });

        let resp = self
            .http
            .post(format!("{KICK_API_BASE}/api/v2/channels/announcements"))
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            let json: serde_json::Value = resp.json().await.map_err(|e| {
                tracing::error!("Failed to parse successful API response: {e}");
                ProviderError::Api(format!("Failed to parse response: {e}"))
            })?;
            let announcement_id = json["data"]["id"]
                .as_str()
                .or_else(|| json["id"].as_str())
                .ok_or_else(|| {
                    tracing::error!("API returned 200 but response missing 'id' field");
                    ProviderError::Api("Missing post ID in API response".into())
                })?
                .to_string();

            Ok(PublishResult {
                platform_post_id: announcement_id,
                platform_post_url: Some(format!("https://kick.com/channel/{channel_id}")),
                status: "published".into(),
            })
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else if status == 429 {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = json["error"]
                .as_str()
                .or_else(|| json["message"].as_str())
                .unwrap_or("Rate limited by Kick API")
                .to_string();
            Err(ProviderError::RateLimited(msg))
        } else {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = json["error"]
                .as_str()
                .or_else(|| json["message"].as_str())
                .unwrap_or("Kick API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Return the authenticated user as a single page.
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let users = self.get_users(access_token).await?;
        let user_data = users["data"].as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or_default();

        Ok(vec![PageInfo {
            id: user_data["id"].as_str().unwrap_or("").to_string(),
            name: user_data["name"]
                .as_str()
                .or_else(|| user_data["display_name"].as_str())
                .unwrap_or("Kick User")
                .to_string(),
            access_token: Some(access_token.to_string()),
            picture: user_data["profile_image"]
                .as_str()
                .or_else(|| user_data["avatar"].as_str())
                .map(String::from),
            username: user_data["username"].as_str().map(String::from),
        }])
    }

    /// Fetch page info by user ID.
    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get(format!("{KICK_API_BASE}/api/v2/users"))
            .query(&[("id", page_id)])
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["error"]
                .as_str()
                .or_else(|| json["message"].as_str())
                .unwrap_or("Failed to fetch Kick user")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let user_data = json["data"].as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or_default();

        Ok(PageInfo {
            id: user_data["id"].as_str().unwrap_or("").to_string(),
            name: user_data["name"]
                .as_str()
                .or_else(|| user_data["display_name"].as_str())
                .unwrap_or("Kick User")
                .to_string(),
            access_token: Some(access_token.to_string()),
            picture: user_data["profile_image"]
                .as_str()
                .or_else(|| user_data["avatar"].as_str())
                .map(String::from),
            username: user_data["username"].as_str().map(String::from),
        })
    }

    async fn comment(
        &self,
        _access_token: &str,
        _post_id: &str,
        _last_comment_id: Option<&str>,
        _post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        Err(ProviderError::Api(
            "Kick does not support programmatic commenting".into(),
        ))
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Kick token expired. Re-authenticate via Kick OAuth.".into())
        } else if status == 429 {
            Some("Kick API rate limit exceeded. Try again later.".into())
        } else {
            None
        }
    }
}
