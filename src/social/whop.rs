// ─── Whop Provider ──────────────────────────────────────────
// Uses Whop OAuth 2.0 + Whop API for announcements.
// Supports: OAuth flow, user info, announcement publishing.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct WhopProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl WhopProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("whop").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SocialProvider for WhopProvider {
    fn identifier(&self) -> &'static str {
        "whop"
    }

    fn name(&self) -> &'static str {
        "Whop"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "read:profile".into(),
            "read:community".into(),
            "write:announcement".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        2000
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
            ("scope", scope.as_str()),
            ("redirect_uri", redirect_uri),
            ("state", state),
            ("response_type", "code"),
        ];

        let url = url::Url::parse_with_params(
            "https://api.whop.com/oauth/authorize",
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
            .post("https://api.whop.com/oauth/token")
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

        // Fetch user info
        let user_resp = self
            .http
            .get("https://api.whop.com/api/v1/me")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let user_status = user_resp.status();
        let user_info: serde_json::Value = user_resp.json().await?;
        if !user_status.is_success() {
            return Err(ProviderError::Api("Failed to fetch Whop user info".into()));
        }

        let user_id = user_info["id"].as_str().unwrap_or("").to_string();
        let email = user_info["email"].as_str().unwrap_or("").to_string();
        let username = user_info["username"].as_str().unwrap_or("").to_string();
        let profile_image = user_info["profile_image_url"]
            .as_str()
            .map(String::from);

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: user_id,
            name: email,
            username,
            picture: profile_image,
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
            .post("https://api.whop.com/oauth/token")
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

    /// Publish an announcement.
    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let title = post
            .settings
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let body = serde_json::json!({
            "title": title,
            "content": post.content,
        });

        let resp = self
            .http
            .post("https://api.whop.com/api/v1/announcements")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            let json: serde_json::Value = resp.json().await?;
            let announcement_id = json["id"]
                .as_str()
                .or_else(|| json["announcement"]["id"].as_str())
                .unwrap_or("")
                .to_string();

            Ok(PublishResult {
                platform_post_id: announcement_id,
                platform_post_url: None,
                status: "published".into(),
            })
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = json["error"]
                .as_str()
                .or_else(|| json["message"].as_str())
                .unwrap_or("Whop API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Return the authenticated user as a single page.
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get("https://api.whop.com/api/v1/me")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if !status.is_success() {
            let msg = json["error"]
                .as_str()
                .unwrap_or("Failed to fetch Whop user")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        Ok(vec![PageInfo {
            id: json["id"].as_str().unwrap_or("").to_string(),
            name: json["email"].as_str().unwrap_or("Whop User").to_string(),
            access_token: Some(access_token.to_string()),
            picture: json["profile_image_url"].as_str().map(String::from),
            username: json["username"].as_str().map(String::from),
        }])
    }

    /// Fetch page info.
    async fn fetch_page_info(
        &self,
        access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        // Whop is single-user, return the authenticated user's info
        self.pages(access_token)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::Api("No Whop user found".into()))
    }

    async fn comment(
        &self,
        _access_token: &str,
        _post_id: &str,
        _last_comment_id: Option<&str>,
        _post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        Err(ProviderError::Api(
            "Whop does not support programmatic commenting".into(),
        ))
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Whop token expired. Re-authenticate via Whop OAuth.".into())
        } else {
            None
        }
    }
}
