// ─── YouTube Provider (Stub) ───────────────────────────────────
// Uses Google OAuth 2.0 + YouTube Data API v3.
// Full implementation requires: OAuth flow, channel selection, video upload.
// Current state: Basic auth flow + info retrieval.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct YoutubeProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl YoutubeProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("youtube").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SocialProvider for YoutubeProvider {
    fn identifier(&self) -> &'static str {
        "youtube"
    }

    fn name(&self) -> &'static str {
        "YouTube"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "https://www.googleapis.com/auth/youtube".into(),
            "https://www.googleapis.com/auth/youtube.upload".into(),
            "https://www.googleapis.com/auth/youtube.force-ssl".into(),
            "https://www.googleapis.com/auth/userinfo.profile".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        5000
    }

    fn is_between_steps(&self) -> bool {
        true
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        // Google OAuth 2.0
        let scope = self.scopes().join(" ");
        let params: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("scope", scope.as_str()),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ];

        let url = url::Url::parse_with_params(
            "https://accounts.google.com/o/oauth2/v2/auth",
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
            ("code", code),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
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
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
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
            name: user["name"].as_str().unwrap_or("").to_string(),
            username: user["email"].as_str().unwrap_or("").to_string(),
            picture: user["picture"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
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

    /// List channels for page selection
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get("https://www.googleapis.com/youtube/v3/channels")
            .query(&[
                ("part", "snippet"),
                ("mine", "true"),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let items = json["items"].as_array().cloned().unwrap_or_default();

        Ok(items
            .iter()
            .map(|item| PageInfo {
                id: item["id"].as_str().unwrap_or("").to_string(),
                name: item["snippet"]["title"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                access_token: Some(access_token.to_string()),
                picture: item["snippet"]["thumbnails"]["default"]["url"]
                    .as_str()
                    .map(String::from),
                username: item["snippet"]["customUrl"]
                    .as_str()
                    .map(String::from),
            })
            .collect())
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get("https://www.googleapis.com/youtube/v3/channels")
            .query(&[
                ("part", "snippet"),
                ("id", page_id),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        if let Some(item) = json["items"].as_array().and_then(|a| a.first()) {
            Ok(PageInfo {
                id: item["id"].as_str().unwrap_or("").to_string(),
                name: item["snippet"]["title"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                access_token: Some(access_token.to_string()),
                picture: item["snippet"]["thumbnails"]["default"]["url"]
                    .as_str()
                    .map(String::from),
                username: item["snippet"]["customUrl"]
                    .as_str()
                    .map(String::from),
            })
        } else {
            Err(ProviderError::Api("YouTube channel not found".into()))
        }
    }

    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        page_id: &str,
    ) -> Result<ReconnectResult, ProviderError> {
        let info = self.fetch_page_info(access_token, page_id).await?;
        Ok(ReconnectResult {
            id: info.id,
            name: info.name,
            access_token: info.access_token.unwrap_or_default(),
            picture: info.picture,
            username: info.username,
        })
    }

    async fn publish(
        &self,
        _access_token: &str,
        _post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // YouTube video upload requires resumable upload protocol.
        // Full implementation needs multipart/resumable upload for video files.
        Err(ProviderError::Api(
            "YouTube video upload requires additional setup. Coming soon.".into(),
        ))
    }
}
