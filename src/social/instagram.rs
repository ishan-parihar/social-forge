// ─── Instagram Provider ───────────────────────────────────────
// OAuth 2.0 + Instagram Graph API (for Business/Creator accounts).
// Posts to Instagram Business accounts via Facebook's Graph API.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct InstagramProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl InstagramProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("instagram")
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
impl SocialProvider for InstagramProvider {
    fn identifier(&self) -> &'static str {
        "instagram"
    }

    fn name(&self) -> &'static str {
        "Instagram"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "instagram_basic".into(),
            "instagram_content_publish".into(),
            "pages_show_list".into(),
            "business_management".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        2200
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
        // Exchange code for token
        let token_params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("code", code),
        ];

        let resp = self
            .http
            .get("https://graph.facebook.com/v21.0/oauth/access_token")
            .query(&token_params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let short_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Get long-lived token
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

        // Find Instagram Business account
        let accounts: serde_json::Value = self
            .http
            .get(format!("{}/me/accounts", self.graph_url()))
            .query(&[("access_token", &access_token)])
            .send()
            .await?
            .json()
            .await?;

        // For each page, look for Instagram Business account
        if let Some(pages) = accounts["data"].as_array() {
            for page in pages {
                let page_id = page["id"].as_str().unwrap_or("");
                let page_token = page["access_token"].as_str().unwrap_or("");

                let ig: serde_json::Value = self
                    .http
                    .get(format!("{}/{}/instagram_business_account", self.graph_url(), page_id))
                    .query(&[("access_token", page_token)])
                    .send()
                    .await?
                    .json()
                    .await?;

                if let Some(ig_obj) = ig["instagram_business_account"].as_object() {
                    let ig_id = ig_obj["id"].as_str().unwrap_or("").to_string();
                    let name = ig_obj["username"].as_str().unwrap_or("").to_string();
                    let profile_pic = ig_obj["profile_picture_url"].as_str().map(String::from);

                    return Ok(AuthToken {
                        access_token: page_token.to_string(),
                        refresh_token: None,
                        expires_in,
                        provider_user_id: ig_id,
                        name: name.clone(),
                        username: name,
                        picture: profile_pic,
                    });
                }
            }
        }

        Err(ProviderError::Auth(
            "No Instagram Business account found. \
             Link Instagram to a Facebook Page first, then reconnect."
                .into(),
        ))
    }

    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Instagram tokens last 60 days. Reconnect the channel.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Resolve the IG Business Account ID from the page-scoped token
        let ig_id = self.resolve_ig_business_account(access_token).await
            .map_err(|e| ProviderError::Api(format!("Cannot resolve IG business account: {e}")))?;

        // Instagram requires media (image/video) for Feed posts
        // For MVP: post a caption-only container if no media provided
        let container_resp = if !post.media.is_empty() {
            // Upload first media item as image container
            let media_url = &post.media[0].url;
            self.http
                .post(format!("{}/{}/media", self.graph_url(), ig_id))
                .form(&[
                    ("image_url", media_url.as_str()),
                    ("caption", post.content.as_str()),
                    ("access_token", access_token),
                ])
                .send()
                .await?
        } else {
            // Text-only not ideal for IG but post as IMAGE type with just caption
            self.http
                .post(format!("{}/{}/media", self.graph_url(), ig_id))
                .form(&[
                    ("media_type", "IMAGE"),
                    ("caption", post.content.as_str()),
                    ("access_token", access_token),
                ])
                .send()
                .await?
        };

        let container_json: serde_json::Value = container_resp.json().await?;
        
        // Check for container creation errors
        if let Some(err) = container_json["error"].as_object() {
            let msg = err["message"].as_str().unwrap_or("Container creation failed");
            return Err(ProviderError::Api(msg.to_string()));
        }

        let container_id = match container_json["id"].as_str() {
            Some(id) => id.to_string(),
            None => return Err(ProviderError::Api(
                format!("Instagram did not return a container ID: {:?}", container_json)
            )),
        };

        // Publish the container
        let publish_resp = self
            .http
            .post(format!("{}/{}/media_publish", self.graph_url(), ig_id))
            .form(&[
                ("creation_id", container_id.as_str()),
                ("access_token", access_token),
            ])
            .send()
            .await?;

        let publish_json: serde_json::Value = publish_resp.json().await?;

        if let Some(err) = publish_json["error"].as_object() {
            let msg = err["message"].as_str().unwrap_or("Publish failed");
            return Err(ProviderError::Api(msg.to_string()));
        }

        if let Some(post_id) = publish_json["id"].as_str() {
            Ok(PublishResult {
                platform_post_id: post_id.to_string(),
                platform_post_url: Some(format!("https://instagram.com/p/{post_id}")),
                status: "published".into(),
            })
        } else {
            Err(ProviderError::Api(
                format!("Instagram publish unexpected response: {:?}", publish_json)
            ))
        }
    }
}

impl InstagramProvider {
    /// Resolve the Instagram Business Account ID from a page-scoped token.
    async fn resolve_ig_business_account(&self, access_token: &str) -> Result<String, String> {
        // Get the user's Facebook pages
        let accounts: serde_json::Value = self
            .http
            .get(format!("{}/me/accounts", self.graph_url()))
            .query(&[("access_token", access_token)])
            .send()
            .await
            .map_err(|e| format!("Failed to get Facebook pages: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse pages response: {e}"))?;

        let pages = accounts["data"].as_array()
            .ok_or_else(|| "No Facebook pages found for this token.".to_string())?;

        for page in pages {
            let page_id = page["id"].as_str().unwrap_or("");
            let page_token = page["access_token"].as_str().unwrap_or(access_token);

            let ig: serde_json::Value = self
                .http
                .get(format!("{}/{page_id}/instagram_business_account", self.graph_url()))
                .query(&[("access_token", page_token)])
                .send()
                .await
                .map_err(|e| format!("Failed to check IG for page {page_id}: {e}"))?
                .json()
                .await
                .map_err(|e| format!("Failed to parse IG response: {e}"))?;

            if let Some(ig_obj) = ig.as_object() {
                if let Some(ig_id) = ig_obj.get("id").and_then(|v| v.as_str()) {
                    // Check for expected structure
                    if ig_obj.contains_key("instagram_business_account") {
                        if let Some(biz) = ig_obj["instagram_business_account"].as_object() {
                            if let Some(id) = biz.get("id").and_then(|v| v.as_str()) {
                                return Ok(id.to_string());
                            }
                        }
                    } else if ig_obj.contains_key("id") {
                        // Direct response from /{page_id}/instagram_business_account includes id at top level
                        return Ok(ig_id.to_string());
                    }
                }
            }
        }

        Err("No Instagram Business account found. Link Instagram to a Facebook Page first.".to_string())
    }
}
