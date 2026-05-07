// ─── LinkedIn Provider ────────────────────────────────────────
// OAuth 2.0 PKCE. Posts to LinkedIn profile or page.
// Uses LinkedIn API v2 (https://api.linkedin.com/v2).

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct LinkedInProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl LinkedInProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("linkedin")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn token_url(&self) -> &'static str {
        "https://www.linkedin.com/oauth/v2/accessToken"
    }

    fn authorize_url(&self) -> &'static str {
        "https://www.linkedin.com/oauth/v2/authorization"
    }
}

#[async_trait]
impl SocialProvider for LinkedInProvider {
    fn identifier(&self) -> &'static str {
        "linkedin"
    }

    fn name(&self) -> &'static str {
        "LinkedIn"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "openid".into(),
            "profile".into(),
            "email".into(),
            "w_member_social".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        3000
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", redirect_uri),
            ("scope", &self.scopes().join(" ")),
            ("state", state),
        ];

        let url = url::Url::parse_with_params(self.authorize_url(), &params)
            .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;

        Ok(AuthUrlResponse { url: url.to_string() })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // LinkedIn uses client_secret_post, not PKCE
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let resp = self.http.post(self.token_url()).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        // Get user profile
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let sub = profile["sub"].as_str().unwrap_or("").to_string();
        let name = profile["name"].as_str().unwrap_or("").to_string();
        let username = profile["preferred_username"].as_str().unwrap_or("").to_string();
        let picture = profile["picture"].as_str().map(String::from);

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: sub,
            name,
            username,
            picture,
        })
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let resp = self.http.post(self.token_url()).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;

        Ok(AuthToken {
            access_token: json["access_token"]
                .as_str()
                .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
                .to_string(),
            refresh_token: json["refresh_token"].as_str().map(String::from),
            expires_in: json["expires_in"].as_u64().map(|v| v as u32),
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
        // Get user profile ID for posting
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let user_id = profile["sub"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Could not get user profile".into()))?;

        let body = serde_json::json!({
            "author": format!("urn:li:person:{user_id}"),
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": post.content,
                    },
                    "shareMediaCategory": if post.media.is_empty() { "NONE" } else { "IMAGE" },
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC"
            }
        });

        let resp = self
            .http
            .post("https://api.linkedin.com/v2/ugcPosts")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == 201 {
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let post_id = location.rsplit('/').next().unwrap_or("").to_string();
            return Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: None, // LinkedIn doesn't always provide URL
                status: "published".into(),
            });
        }

        let json: serde_json::Value = resp.json().await?;

        if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .or_else(|| json["error_description"].as_str())
                .unwrap_or("LinkedIn publish failed")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }
}
