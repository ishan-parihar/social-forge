// ─── Twitch Provider ────────────────────────────────────────
// Uses Twitch OAuth 2.0 + Helix API for channel announcements.
// Supports: OAuth flow, channel announcements, user info.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct TwitchProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl TwitchProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("twitch").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    /// Fetch the authenticated user's info from Helix.
    async fn get_users(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://api.twitch.tv/helix/users")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Client-Id", self.client_id.as_str())
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("Unknown Twitch API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }
}

#[async_trait]
impl SocialProvider for TwitchProvider {
    fn identifier(&self) -> &'static str {
        "twitch"
    }

    fn name(&self) -> &'static str {
        "Twitch"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "user:read:email".into(),
            "channel:manage:channel_posts".into(),
            "channel:read:subscriptions".into(),
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
            ("force_verify", "true"),
        ];

        let url = url::Url::parse_with_params(
            "https://id.twitch.tv/oauth2/authorize",
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
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .http
            .post("https://id.twitch.tv/oauth2/token")
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["message"]
                .as_str()
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

        // Fetch user info from Helix
        let user_info = self.get_users(&access_token).await?;
        let user = user_info["data"][0].clone();
        let provider_user_id = user["id"].as_str().unwrap_or("").to_string();
        let display_name = user["display_name"].as_str().unwrap_or("").to_string();
        let login = user["login"].as_str().unwrap_or("").to_string();
        let profile_image_url = user["profile_image_url"].as_str().map(String::from);

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id,
            name: display_name,
            username: login,
            picture: profile_image_url,
        })
    }

    async fn refresh_token(&self, token: &str) -> Result<AuthToken, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", token),
        ];

        let resp = self
            .http
            .post("https://id.twitch.tv/oauth2/token")
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["message"]
                .as_str()
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

    /// Publish a chat announcement to the broadcaster's channel.
    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Get the user ID from the integration's internal_id or fetch via /users
        let user_id = post
            .settings
            .get("moderator_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Fall back to fetching from /users endpoint
                "".to_string()
            });

        let broadcaster_id = if user_id.is_empty() {
            // Fetch from Helix /users
            let users = self.get_users(access_token).await?;
            users["data"][0]["id"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else {
            user_id.clone()
        };

        if broadcaster_id.is_empty() {
            return Err(ProviderError::Api(
                "Could not determine broadcaster ID for Twitch announcement".into(),
            ));
        }

        let body = serde_json::json!({
            "message": post.content,
        });

        let resp = self
            .http
            .post("https://api.twitch.tv/helix/chat/announcements")
            .query(&[("broadcaster_id", &broadcaster_id), ("moderator_id", &broadcaster_id)])
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Client-Id", self.client_id.as_str())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            Ok(PublishResult {
                platform_post_id: String::new(),
                platform_post_url: Some(format!("https://www.twitch.tv/popout/{broadcaster_id}/chat")),
                status: "published".into(),
            })
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else if status == 429 {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = json["message"]
                .as_str()
                .unwrap_or("Rate limited by Twitch API")
                .to_string();
            Err(ProviderError::RateLimited(msg))
        } else {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = json["message"]
                .as_str()
                .unwrap_or("Twitch API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Return the authenticated user as a single page.
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let users = self.get_users(access_token).await?;
        let user = users["data"][0].clone();

        Ok(vec![PageInfo {
            id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["display_name"].as_str().unwrap_or("").to_string(),
            access_token: Some(access_token.to_string()),
            picture: user["profile_image_url"].as_str().map(String::from),
            username: user["login"].as_str().map(String::from),
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
            .get("https://api.twitch.tv/helix/users")
            .query(&[("id", page_id)])
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Client-Id", self.client_id.as_str())
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let msg = json["message"]
                .as_str()
                .unwrap_or("Failed to fetch Twitch user")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let user = json["data"][0].clone();

        Ok(PageInfo {
            id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["display_name"].as_str().unwrap_or("").to_string(),
            access_token: Some(access_token.to_string()),
            picture: user["profile_image_url"].as_str().map(String::from),
            username: user["login"].as_str().map(String::from),
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
            "Twitch API does not support programmatic commenting".into(),
        ))
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Twitch token expired. Re-authenticate via Twitch OAuth.".into())
        } else if status == 429 {
            Some("Twitch API rate limit exceeded. Try again later.".into())
        } else {
            None
        }
    }
}
