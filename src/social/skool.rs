use async_trait::async_trait;
use serde_json::{json, Value};

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

    /// Fetch the Next.js buildId from a Skool community page.
    /// Required for constructing Next.js data route URLs.
    async fn get_build_id(&self, slug: &str, access_token: &str) -> Result<String, ProviderError> {
        let url = format!("https://www.skool.com/{}", slug);
        let response = self.http.get(&url)
            .header("Cookie", format!("auth_token={}", access_token))
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let html = response.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        // Extract buildId from __NEXT_DATA__ JSON embedded in HTML
        let marker = "\"buildId\":\"";
        let start = html.find(marker).ok_or_else(|| ProviderError::Api("buildId not found in page".into()))?;
        let value_start = start + marker.len();
        let value_end = html[value_start..].find('"').ok_or_else(|| ProviderError::Api("buildId malformed".into()))?;
        Ok(html[value_start..value_start + value_end].to_string())
    }

    /// Get community info (name, description, member count, etc.)
    pub async fn get_community_info(&self, slug: &str, access_token: &str) -> Result<Value, ProviderError> {
        let build_id = self.get_build_id(slug, access_token).await?;
        let url = format!("https://www.skool.com/_next/data/{}/{}/about.json", build_id, slug);
        let response = self.http.get(&url)
            .header("Cookie", format!("auth_token={}", access_token))
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(json!({"raw": text}));
        if status.is_success() { Ok(v) }
        else { Err(ProviderError::Api(v["message"].as_str().unwrap_or(&text).into())) }
    }

    /// List posts in a community, with optional pagination/sort/category filter
    pub async fn list_posts(&self, slug: &str, access_token: &str, page: Option<u32>, sort: Option<&str>, category: Option<&str>) -> Result<Value, ProviderError> {
        let build_id = self.get_build_id(slug, access_token).await?;
        let mut url = format!("https://www.skool.com/_next/data/{}/{}.json", build_id, slug);
        let mut params: Vec<String> = Vec::new();
        if let Some(p) = page { params.push(format!("p={}", p)); }
        if let Some(s) = sort { params.push(format!("s={}", s)); }
        if let Some(c) = category { params.push(format!("c={}", c)); }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        let response = self.http.get(&url)
            .header("Cookie", format!("auth_token={}", access_token))
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(json!({"raw": text}));
        if status.is_success() { Ok(v) }
        else { Err(ProviderError::Api(v["message"].as_str().unwrap_or(&text).into())) }
    }

    /// Get a single post by community slug and post slug
    pub async fn get_post(&self, slug: &str, post_slug: &str, access_token: &str) -> Result<Value, ProviderError> {
        let build_id = self.get_build_id(slug, access_token).await?;
        let url = format!("https://www.skool.com/_next/data/{}/{}/p/{}.json", build_id, slug, post_slug);
        let response = self.http.get(&url)
            .header("Cookie", format!("auth_token={}", access_token))
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(json!({"raw": text}));
        if status.is_success() { Ok(v) }
        else { Err(ProviderError::Api(v["message"].as_str().unwrap_or(&text).into())) }
    }

    /// Create a comment on a post via api2.skool.com
    pub async fn create_comment(&self, post_id: &str, group_id: &str, content: &str, access_token: &str) -> Result<Value, ProviderError> {
        let url = format!("{}/comments", self.api_url(""));
        let body = json!({
            "post_id": post_id,
            "group_id": group_id,
            "metadata": {"content": content}
        });
        let response = self.http.post(&url)
            .header("Content-Type", "application/json")
            .header("Cookie", format!("auth_token={}", access_token))
            .json(&body)
            .send().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(json!({"raw": text}));
        if status.is_success() { Ok(v) }
        else { Err(ProviderError::Api(v["message"].as_str().unwrap_or(&text).into())) }
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

        let content = common::strip_html_tags(&post.content);

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
        // Shared length + capability check first.
        super::validate_post_rules(self.name(), self.max_content_length(), &post.content)?;

        // `publish` reads both keys straight out of settings and silently
        // substitutes empty strings, which produces a 4xx from api2.skool.com
        // only at publish time. Catch both here so staging can warn instead.
        for (key, why) in [
            ("title", "Skool renders every post with a title line"),
            ("groupId", "groupId selects the community to post into"),
        ] {
            let value = post.settings.get(key).and_then(|v| v.as_str()).unwrap_or("");
            if value.trim().is_empty() {
                return Err(format!(
                    "Skool posts require a non-empty '{key}' in settings — {why}, and a post \
                     without it cannot be published."
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::test_config;

    fn post(settings: serde_json::Value) -> PostContent {
        PostContent {
            content: "body".into(),
            settings,
            ..Default::default()
        }
    }

    /// `publish` reads `title`/`groupId` out of settings and substitutes empty
    /// strings, so a missing key only fails as a 4xx at publish time. Both
    /// must be caught by `validate_post` while staging still holds the post.
    #[test]
    fn validate_post_requires_title_then_group_id() {
        let p = SkoolProvider::new();

        let no_settings = p.validate_post(&post(serde_json::json!({}))).unwrap_err();
        assert!(no_settings.contains("'title'"), "got: {no_settings}");

        let no_group = p
            .validate_post(&post(serde_json::json!({ "title": "T" })))
            .unwrap_err();
        assert!(no_group.contains("'groupId'"), "got: {no_group}");

        let blank_group = p
            .validate_post(&post(serde_json::json!({ "title": "T", "groupId": "  " })))
            .unwrap_err();
        assert!(blank_group.contains("'groupId'"), "got: {blank_group}");

        assert!(p
            .validate_post(&post(serde_json::json!({ "title": "T", "groupId": "g1" })))
            .is_ok());
    }

    /// The shared length rule still runs before the Skool-specific ones, so
    /// an over-long body is not reported as a missing title.
    #[test]
    fn validate_post_reports_length_before_settings() {
        let p = SkoolProvider::new();
        let long = PostContent {
            content: "a".repeat(20_000),
            settings: serde_json::json!({}),
            ..Default::default()
        };
        assert!(p.validate_post(&long).unwrap_err().contains("too long"));
    }

    /// A provider with a zero content limit has no post surface at all
    /// (GitHub: issues/PRs only). `publish_capable` and `validate_post` must
    /// both say so instead of quoting "Maximum is 0 chars".
    #[test]
    fn zero_content_length_means_not_publish_capable() {
        let gh = crate::social::github::GithubProvider::new(&test_config());
        assert!(!gh.publish_capable());
        let err = gh
            .validate_post(&post(serde_json::json!({ "title": "T", "groupId": "g" })))
            .unwrap_err();
        assert!(err.contains("no post-publish surface"), "got: {err}");

        let skool = SkoolProvider::new();
        assert!(skool.publish_capable());
    }
}
