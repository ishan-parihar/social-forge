// ─── X/Twitter Provider ───────────────────────────────────────
// Uses OAuth 2.0 PKCE via Twitter API v2.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct XProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl XProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("x")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn oauth_token_endpoint(&self) -> &'static str {
        "https://api.twitter.com/2/oauth2/token"
    }

    fn oauth_authorize_endpoint(&self) -> &'static str {
        "https://twitter.com/i/oauth2/authorize"
    }
}

#[async_trait]
impl SocialProvider for XProvider {
    fn identifier(&self) -> &'static str {
        "x"
    }

    fn name(&self) -> &'static str {
        "X (Twitter)"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "tweet.read".into(),
            "tweet.write".into(),
            "users.read".into(),
            "offline.access".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        4000
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let challenge = common::generate_code_challenge(code_verifier);

        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", redirect_uri),
            ("scope", &self.scopes().join(" ")),
            ("state", state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ];

        let url = url::Url::parse_with_params(self.oauth_authorize_endpoint(), &params)
            .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;

        Ok(AuthUrlResponse {
            url: url.to_string(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        let json = common::exchange_code_for_token(
            &self.http,
            self.oauth_token_endpoint(),
            &self.client_id,
            &self.client_secret,
            code,
            code_verifier,
            redirect_uri,
        )
        .await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token in response".into()))?
            .to_string();

        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        // Get user info
        let user_info = self
            .http
            .get("https://api.twitter.com/2/users/me")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let provider_user_id = user_info["data"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let name = user_info["data"]["name"].as_str().unwrap_or("").to_string();
        let username = user_info["data"]["username"].as_str().unwrap_or("").to_string();

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id,
            name,
            username,
            picture: None,
        })
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        let json = common::refresh_access_token(
            &self.http,
            self.oauth_token_endpoint(),
            &self.client_id,
            &self.client_secret,
            refresh_token,
        )
        .await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        let new_refresh = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        Ok(AuthToken {
            access_token,
            refresh_token: new_refresh,
            expires_in,
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let mut body = serde_json::json!({
            "text": post.content,
        });

        // Add media if present
        if !post.media.is_empty() {
            let media_ids = self.upload_media(access_token, &post.media).await?;
            if !media_ids.is_empty() {
                body["media"] = serde_json::json!({ "media_ids": media_ids });
            }
        }

        let resp = self
            .http
            .post("https://api.twitter.com/2/tweets")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 201 || status == 200 {
            let post_id = json["data"]["id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            Ok(PublishResult {
                platform_post_url: Some(format!("https://twitter.com/user/status/{post_id}")),
                platform_post_id: post_id.clone(),
                status: "published".into(),
            })
        } else if status == 429 {
            Err(ProviderError::RateLimited("X API rate limit".into()))
        } else if json["title"].as_str() == Some("Unauthorized")
            || json["status"].as_i64() == Some(401)
        {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["detail"].as_str().unwrap_or("Unknown error").to_string(),
            ))
        }
    }
}

impl XProvider {
    async fn upload_media(
        &self,
        _access_token: &str,
        _media: &[MediaAttachment],
    ) -> Result<Vec<String>, ProviderError> {
        // X/Twitter media upload requires chunked upload via
        // media/upload (async) endpoint. Simplified for MVP.
        Ok(vec![])
    }
}
