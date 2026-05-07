// ─── Facebook Provider ────────────────────────────────────────
// OAuth 2.0 + Graph API. Posts to Facebook Page (not personal timeline).

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct FacebookProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl FacebookProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("facebook")
            .unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    fn graph_url(&self) -> &'static str {
        "https://graph.facebook.com/v21.0"
    }
}

#[async_trait]
impl SocialProvider for FacebookProvider {
    fn identifier(&self) -> &'static str {
        "facebook"
    }

    fn name(&self) -> &'static str {
        "Facebook"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "pages_show_list".into(),
            "pages_read_engagement".into(),
            "pages_manage_posts".into(),
            "public_profile".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        63206
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
            ("scope", scope.as_str()),
            ("state", state),
            ("response_type", "code"),
        ];

        let url = url::Url::parse_with_params(
            "https://www.facebook.com/v21.0/dialog/oauth",
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
        // Exchange code for access token
        let token_params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("code", code),
        ];

        let token_resp = self
            .http
            .get("https://graph.facebook.com/v21.0/oauth/access_token")
            .query(&token_params)
            .send()
            .await?;

        let token_json: serde_json::Value = token_resp.json().await?;
        let short_token = token_json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Exchange short-lived token for long-lived token
        let long_params: Vec<(&str, &str)> = vec![
            ("grant_type", "fb_exchange_token"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("fb_exchange_token", short_token.as_str()),
        ];

        let long_resp = self
            .http
            .get("https://graph.facebook.com/v21.0/oauth/access_token")
            .query(&long_params)
            .send()
            .await?;

        let long_json: serde_json::Value = long_resp.json().await?;
        let access_token = long_json["access_token"]
            .as_str()
            .unwrap_or(&short_token)
            .to_string();
        let expires_in = long_json["expires_in"].as_u64().map(|v| v as u32);

        // Get user's pages
        let pages: serde_json::Value = self
            .http
            .get(format!("{}/me/accounts", self.graph_url()))
            .query(&[("access_token", &access_token)])
            .send()
            .await?
            .json()
            .await?;

        // Pick first page (MVP). Multi-page support later.
        if let Some(page) = pages["data"].as_array().and_then(|a| a.first()) {
            let page_id = page["id"].as_str().unwrap_or("").to_string();
            let page_name = page["name"].as_str().unwrap_or("").to_string();
            let page_token = page["access_token"].as_str().unwrap_or("").to_string();
            let picture = page["picture"]["data"]["url"].as_str().map(String::from);

            let name = page_name.clone();
            return Ok(AuthToken {
                access_token: page_token,
                refresh_token: None,
                expires_in,
                provider_user_id: page_id,
                name,
                username: page_name,
                picture,
            });
        }

        Err(ProviderError::Auth("No Facebook pages found. Create a Page first.".into()))
    }

    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Facebook long-lived tokens last 60 days. Reconnect the channel.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Use the page-scoped token's associated page ID for posting.
        // The page_id is stored in the token itself or resolved from /me/accounts.
        let page_id = self.resolve_page_id(access_token).await?;

        let params = vec![
            ("message", post.content.as_str()),
            ("access_token", access_token),
        ];

        let resp = self
            .http
            .post(format!("{}/{page_id}/feed", self.graph_url()))
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 200 {
            let post_id = json["id"].as_str().unwrap_or("").to_string();
            Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: None,
                status: "published".into(),
            })
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            Err(ProviderError::Api(
                json["error"]["message"]
                    .as_str()
                    .unwrap_or("Facebook publish failed")
                    .to_string(),
            ))
        }
    }
}

impl FacebookProvider {
    /// Resolve the page ID associated with a page-scoped access token.
    async fn resolve_page_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let me: serde_json::Value = self
            .http
            .get(format!("{}/me/accounts", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await?
            .json()
            .await?;

        let pages = me["data"].as_array()
            .ok_or_else(|| ProviderError::Auth("No pages found for token. Ensure you have a Facebook Page.".into()))?;

        pages.first()
            .and_then(|page| page["id"].as_str().map(String::from))
            .ok_or_else(|| ProviderError::Auth("Could not resolve page ID from token.".into()))
    }
}
