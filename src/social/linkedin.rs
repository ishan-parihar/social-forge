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

    // ─── API Methods ──────────────────────────────────────────

    pub async fn get_profile(&self, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let url = "https://api.linkedin.com/v2/userinfo";
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
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
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_user_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let profile = self.get_profile(access_token).await?;
        let user_id = profile["sub"]
            .as_str()
            .ok_or_else(|| ProviderError::Api("Could not get LinkedIn user ID".into()))?
            .to_string();
        Ok(user_id)
    }

    pub async fn get_posts(
        &self,
        access_token: &str,
        author_urn: &str,
        limit: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.clamp(1, 100);
        let url = format!(
            "https://api.linkedin.com/v2/rest/posts?author={author_urn}&count={limit}"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
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
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_post_detail(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.linkedin.com/v2/rest/posts/{post_urn}");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
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
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_post_comments(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/comments"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
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
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn delete_post(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<(), ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/posts/{}",
            urlencoding::encode(post_urn)
        );
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() || status.as_u16() == 204 {
            Ok(())
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ProviderError::Api(format!("Delete failed ({}): {}", status, body)))
        }
    }

    pub async fn get_reactions(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/likes"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("LinkedIn API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn get_shares(
        &self,
        access_token: &str,
        post_urn: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/shares"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("LinkedIn API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn create_comment(
        &self,
        access_token: &str,
        post_urn: &str,
        actor_urn: &str,
        message: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{post_urn}/comments"
        );
        let body = serde_json::json!({
            "actor": actor_urn,
            "message": {
                "text": message,
            },
            "object": post_urn,
        });
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status == 201 {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn API error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
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

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        // Fetch profile info so it's not lost on refresh
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(AuthToken {
            access_token,
            refresh_token: json["refresh_token"].as_str().map(String::from),
            expires_in: json["expires_in"].as_u64().map(|v| v as u32),
            provider_user_id: profile["sub"].as_str().unwrap_or("").to_string(),
            name: profile["name"].as_str().unwrap_or("").to_string(),
            username: profile["preferred_username"].as_str().unwrap_or("").to_string(),
            picture: profile["picture"].as_str().map(String::from),
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

    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        _page_id: &str,
    ) -> Result<super::ReconnectResult, ProviderError> {
        let profile = self
            .http
            .get("https://api.linkedin.com/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(super::ReconnectResult {
            id: profile["sub"].as_str().unwrap_or("").to_string(),
            name: profile["name"].as_str().unwrap_or("").to_string(),
            access_token: access_token.to_string(),
            picture: profile["picture"].as_str().map(String::from),
            username: profile["preferred_username"].as_str().map(String::from),
        })
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        // LinkedIn personal profiles: get network size (connections/followers)
        let user_id = self.get_user_id(access_token).await?;
        let person_urn = format!("urn:li:person:{user_id}");

        let mut results = Vec::new();

        // Get follower/connection count
        let url = format!(
            "https://api.linkedin.com/v2/networkSizes/{}?edgeType=CompanyFollowedByMember",
            person_urn
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(count) = json["firstDegreeSize"].as_u64() {
                results.push(AnalyticsData {
                    label: "connections".into(),
                    data: vec![AnalyticsDataPoint {
                        total: count.to_string(),
                        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    }],
                    percentage_change: 0.0,
                });
            }
        }

        Ok(results)
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        // Get social actions (likes, comments, shares counts) for a post
        let url = format!(
            "https://api.linkedin.com/v2/rest/socialActions/{platform_post_id}"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if !status.is_success() {
            let msg = json["message"].as_str().unwrap_or("LinkedIn post analytics error").to_string();
            return Err(ProviderError::Api(msg));
        }

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut results = Vec::new();

        if let Some(n) = json["likesSummary"]["totalLikes"].as_u64() {
            results.push(AnalyticsData {
                label: "likes".into(),
                data: vec![AnalyticsDataPoint { total: n.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(n) = json["commentsSummary"]["totalFirstLevelComments"].as_u64() {
            results.push(AnalyticsData {
                label: "comments".into(),
                data: vec![AnalyticsDataPoint { total: n.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(n) = json["sharesSummary"]["totalShares"].as_u64() {
            results.push(AnalyticsData {
                label: "shares".into(),
                data: vec![AnalyticsDataPoint { total: n.to_string(), date: today }],
                percentage_change: 0.0,
            });
        }

        Ok(results)
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api("LinkedIn personal profile does not support page selection".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            database_url: "sqlite:test".into(),
            jwt_secret: "test".into(),
            app_url: "http://localhost:3000".into(),
            frontend_url: "http://localhost:4200".into(),
            x_client_id: None,
            x_client_secret: None,
            x_auth_token: None,
            x_ct0: None,
            linkedin_client_id: Some("test_linkedin_id".into()),
            linkedin_client_secret: Some("test_linkedin_secret".into()),
            bluesky_handle: None,
            bluesky_app_password: None,
            facebook_client_id: None,
            facebook_client_secret: None,
            instagram_client_id: None,
            instagram_client_secret: None,
            threads_app_id: None,
            threads_app_secret: None,
            youtube_client_id: None,
            youtube_client_secret: None,
            reddit_client_id: None,
            reddit_client_secret: None,
            reddit_username: None,
            reddit_password: None,
            reddit_access_token: None,
            reddit_refresh_token: None,
            discord_client_id: None,
            discord_client_secret: None,
            discord_bot_token: None,
            telegram_bot_tokens: None,
            telegram_session_dir: None,
            telegram_api_id: None,
            telegram_api_hash: None,
            tiktok_client_id: None,
            tiktok_client_secret: None,
            medium_access_token: None,
            devto_api_key: None,
            pinterest_client_id: None,
            pinterest_client_secret: None,
            whatsapp_store_dir: None,
            slack_client_id: None,
            slack_client_secret: None,
            instagram_app_id: None,
            instagram_app_secret: None,
            mastodon_client_id: None,
            mastodon_client_secret: None,
            mastodon_instance_url: None,
            hashnode_api_key: None,
            github_token: None,
            twitch_client_id: None,
            twitch_client_secret: None,
            vk_client_id: None,
            vk_client_secret: None,
            whop_client_id: None,
            whop_client_secret: None,
            mewe_client_id: None,
            mewe_client_secret: None,
            moltbook_client_id: None,
            moltbook_client_secret: None,
            kick_client_id: None,
            kick_client_secret: None,
            neynar_api_key: None,
            nostr_private_key: None,
            token_encryption_key: None,
            media_dir: "./uploads".into(),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            stripe_price_free: None,
            stripe_price_pro_monthly: None,
            stripe_price_pro_annual: None,
            stripe_price_business_monthly: None,
            stripe_price_business_annual: None,
        }
    }

    #[test]
    fn test_scopes_contain_required() {
        let provider = LinkedInProvider::new(&test_config());
        let scopes = provider.scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"w_member_social".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
    }

    #[test]
    fn test_identifier_and_name() {
        let provider = LinkedInProvider::new(&test_config());
        assert_eq!(provider.identifier(), "linkedin");
        assert_eq!(provider.name(), "LinkedIn");
    }

    #[test]
    fn test_max_content_length() {
        let provider = LinkedInProvider::new(&test_config());
        assert_eq!(provider.max_content_length(), 3000);
    }

    #[tokio::test]
    async fn test_generate_auth_url_contains_params() {
        let provider = LinkedInProvider::new(&test_config());
        let result = provider.generate_auth_url("test_state", "test_verifier", "http://localhost:3000/callback").await;
        let url = result.unwrap().url;

        assert!(url.contains("response_type=code"), "should contain response_type=code");
        assert!(url.contains("client_id=test_linkedin_id"), "should contain client_id");
        assert!(url.contains("redirect_uri="), "should contain redirect_uri");
        assert!(url.contains("state=test_state"), "should contain state");
        assert!(url.contains("scope="), "should contain scope");
        assert!(url.starts_with("https://www.linkedin.com/oauth/v2/authorization"));
    }
}
