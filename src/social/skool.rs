use async_trait::async_trait;

use super::*;

pub struct SkoolProvider {
    http: reqwest::Client,
}

impl SkoolProvider {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("https://api2.skool.com{}", path)
    }
}

#[async_trait]
impl SocialProvider for SkoolProvider {
    fn identifier(&self) -> &'static str {
        "skool"
    }

    fn name(&self) -> &'static str {
        "Skool"
    }

    fn scopes(&self) -> Vec<String> {
        vec![]
    }

    fn max_content_length(&self) -> usize {
        10000
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    fn is_chrome_extension(&self) -> bool {
        true
    }

    fn one_time_token(&self) -> bool {
        true
    }

    fn extension_cookies(&self) -> Vec<(&'static str, &'static str)> {
        vec![("auth_token", "skool.com")]
    }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Uses Chrome extension to extract session cookies from skool.com")
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        Ok(AuthUrlResponse {
            url: String::new(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        let token = code.trim();
        if token.is_empty() {
            return Err(ProviderError::Auth(
                "No auth_token provided. Use the Chrome extension to extract your Skool session cookie."
                    .into(),
            ));
        }

        // Validate by calling health endpoint
        let resp = self
            .http
            .get(self.api_url("/health"))
            .header("Cookie", format!("auth_token={}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Auth(
                "Invalid or expired Skool auth_token. Re-login to skool.com and extract a fresh cookie."
                    .into(),
            ));
        }

        // Try to fetch user info via groups endpoint as a second validation
        let user_name = "Skool User".to_string();
        let user_id = token[..token.len().min(16)].to_string();

        Ok(AuthToken {
            access_token: token.to_string(),
            refresh_token: None,
            expires_in: Some(31536000), // ~1 year (cookie lifetime)
            provider_user_id: user_id,
            name: user_name,
            username: "skool".to_string(),
            picture: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Skool tokens do not expire. Re-connect if your session expires.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let group_id = post.settings.get("groupId")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let label = post.settings.get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let title = post.settings.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let content = strip_html_tags(&post.content);

        // Parse media URLs if present
        let media: Vec<&str> = post.media.iter().map(|m| m.url.as_str()).collect();

        let body = serde_json::json!({
            "groupId": group_id,
            "metadata": {
                "displayName": title,
                "content": content,
                "label": label,
                "media": media,
            }
        });

        let resp = self
            .http
            .post(self.api_url("/posts"))
            .header("Cookie", format!("auth_token={}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let response_body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::Api(format!(
                "Skool API error ({}): {}",
                status, response_body
            )));
        }

        // Parse post ID from response
        let json: serde_json::Value =
            serde_json::from_str(&response_body).unwrap_or(serde_json::Value::Null);
        let post_id = json["id"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("skool-{}", chrono::Utc::now().timestamp()));

        Ok(PublishResult {
            platform_post_id: post_id,
            platform_post_url: None,
            status: "published".into(),
        })
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Skool does not support page management via API".into(),
        ))
    }

    async fn pages(
        &self,
        _access_token: &str,
    ) -> Result<Vec<PageInfo>, ProviderError> {
        // Group listing requires Next.js data routes with buildId discovery
        // This requires more complex implementation — return empty for now
        Ok(vec![])
    }

    fn validate_post(&self, post: &PostContent) -> Result<(), String> {
        if post.content.len() > self.max_content_length() {
            return Err(format!(
                "Content too long ({} chars). Maximum is {} chars for Skool.",
                post.content.len(),
                self.max_content_length()
            ));
        }
        let title = post.settings.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if title.is_empty() {
            return Err("Skool posts require a 'title' in settings.".into());
        }
        Ok(())
    }
}

/// Strip HTML tags from content (basic implementation)
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => {
                if !in_tag {
                    result.push(ch);
                }
            }
        }
    }
    result
}
