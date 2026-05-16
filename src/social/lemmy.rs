// ─── Lemmy Provider ────────────────────────────────────────────
// Uses Lemmy API v3 with API key authentication.
// Credentials stored as JSON: {"api_key":"...","instance_url":"..."}

use async_trait::async_trait;
use reqwest::StatusCode;

use super::*;
use crate::config::Config;

const DEFAULT_LEMMY_INSTANCE: &str = "https://lemmy.world";

pub struct LemmyProvider {
    http: reqwest::Client,
}

impl LemmyProvider {
    pub fn new(_config: &Config) -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Parse credentials from JSON token: {"api_key":"...","instance_url":"..."}
    fn parse_creds(&self, access_token: &str) -> (String, String) {
        match serde_json::from_str::<serde_json::Value>(access_token) {
            Ok(creds) => {
                let api_key = creds["api_key"].as_str().unwrap_or("").to_string();
                let instance_url = creds["instance_url"]
                    .as_str()
                    .unwrap_or(DEFAULT_LEMMY_INSTANCE)
                    .to_string();
                (api_key, instance_url)
            }
            Err(_) => (String::new(), DEFAULT_LEMMY_INSTANCE.to_string()),
        }
    }

    /// Fetch site info from a Lemmy instance
    async fn fetch_site(&self, instance_url: &str, api_key: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}/api/v3/site", instance_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

        if status.is_success() {
            Ok(json)
        } else {
            let msg = json["error"].as_str().unwrap_or(&body).to_string();
            Err(ProviderError::Api(format!("Lemmy API error ({}): {}", status, msg)))
        }
    }
}

#[async_trait]
impl SocialProvider for LemmyProvider {
    fn identifier(&self) -> &'static str {
        "lemmy"
    }

    fn name(&self) -> &'static str {
        "Lemmy"
    }

    fn scopes(&self) -> Vec<String> {
        vec![] // Lemmy uses API keys, not OAuth scopes
    }

    fn max_content_length(&self) -> usize {
        50000 // ~50KB
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Normal
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Uses API key authentication. Provide your Lemmy instance URL and API key.")
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        Err(ProviderError::Auth(
            "Lemmy uses API key authentication. Provide your instance URL and API key directly."
                .into(),
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // Parse JSON credentials from the code parameter (like WordPress)
        let creds: serde_json::Value = serde_json::from_str(code).map_err(|_| {
            ProviderError::Auth(
                "Invalid credentials format. Expected JSON with api_key and instance_url."
                    .into(),
            )
        })?;

        let api_key = creds["api_key"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing 'api_key' in credentials".into()))?;
        let instance_url = creds["instance_url"]
            .as_str()
            .unwrap_or(DEFAULT_LEMMY_INSTANCE);

        if api_key.is_empty() {
            return Err(ProviderError::Auth("api_key must not be empty.".into()));
        }

        // Validate by fetching site info
        let site = self.fetch_site(instance_url, api_key).await?;

        let site_name = site["site_view"]["site"]["name"]
            .as_str()
            .unwrap_or("Lemmy")
            .to_string();

        // Store credentials as JSON in access_token (WordPress pattern)
        let token_json = serde_json::json!({
            "api_key": api_key,
            "instance_url": instance_url,
        });

        Ok(AuthToken {
            access_token: token_json.to_string(),
            refresh_token: None,
            expires_in: None,
            provider_user_id: "lemmy".to_string(),
            name: site_name.clone(),
            username: site_name.to_lowercase(),
            picture: site["site_view"]["site"]["icon"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Lemmy API keys do not expire. Re-connect if needed.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let (api_key, instance_url) = self.parse_creds(access_token);
        if api_key.is_empty() {
            return Err(ProviderError::Auth(
                "Invalid Lemmy credentials. Re-connect your Lemmy integration.".into(),
            ));
        }

        // Extract title from settings or first line of content
        let title = post
            .settings
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                post.content
                    .lines()
                    .next()
                    .unwrap_or("Untitled")
                    .to_string()
            });

        // Extract optional community/board name from settings
        let community_id = post.settings.get("community_id").and_then(|v| v.as_i64());
        let community_name = post.settings.get("community").and_then(|v| v.as_str());

        // Build the post body for Lemmy API v3
        let mut body = serde_json::json!({
            "name": title,
            "body": post.content,
            "auth": api_key,
        });

        if let Some(cid) = community_id {
            body["community_id"] = serde_json::json!(cid);
        }
        if let Some(cname) = community_name {
            body["community_name"] = serde_json::json!(cname);
        }

        // Extract optional URL (for link posts)
        if let Some(url) = post.settings.get("url").and_then(|v| v.as_str()) {
            body["url"] = serde_json::json!(url);
        }

        // Lemmy API v3 requires either community_id or community_name
        if body.get("community_id").is_none() && body.get("community_name").is_none() {
            // Default to "main" community if none specified
            body["community_name"] = serde_json::json!("main");
        }

        let url = format!(
            "{}/api/v3/post",
            instance_url.trim_end_matches('/')
        );

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status_code = resp.status();
        let response_body = resp.text().await.unwrap_or_default();
        let json: serde_json::Value =
            serde_json::from_str(&response_body).unwrap_or(serde_json::Value::Null);

        if status_code == StatusCode::OK {
            let post_view = &json["post_view"];
            let post_id = post_view["post"]["id"]
                .as_u64()
                .map(|id| id.to_string())
                .unwrap_or_default();
            let post_url = post_view["post"]["ap_id"].as_str().map(String::from);

            Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: post_url,
                status: "published".into(),
            })
        } else {
            let msg = json["error"]
                .as_str()
                .unwrap_or_else(|| {
                    json["message"]
                        .as_str()
                        .unwrap_or("Lemmy publish failed")
                })
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let (api_key, instance_url) = self.parse_creds(access_token);
        if api_key.is_empty() {
            return Err(ProviderError::Auth("Invalid Lemmy credentials".into()));
        }

        let site = self.fetch_site(&instance_url, &api_key).await?;
        let site_view = &site["site_view"]["site"];

        Ok(vec![PageInfo {
            id: "site".to_string(),
            name: site_view["name"]
                .as_str()
                .unwrap_or("Lemmy")
                .to_string(),
            access_token: Some(access_token.to_string()),
            picture: site_view["icon"].as_str().map(String::from),
            username: site_view["actor_id"]
                .as_str()
                .map(|s| s.trim_start_matches("https://").trim_start_matches("http://").to_string()),
        }])
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        // Reuse pages() since Lemmy has no multi-page concept
        let pages = self.pages(access_token).await?;
        pages.into_iter().next().ok_or_else(|| {
            ProviderError::Api("No page info available".into())
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
            "Lemmy does not support commenting via third-party API keys.".into(),
        ))
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Invalid Lemmy API key. Check your credentials and re-connect.".into())
        } else if status == 403 {
            Some("Access denied. Your API key may not have permission for this action.".into())
        } else if status == 429 {
            Some("Lemmy API rate limit exceeded. Try again later.".into())
        } else if status == 404 {
            Some("Lemmy resource not found. Check the community name or URL.".into())
        } else {
            None
        }
    }
}
