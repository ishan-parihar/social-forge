// ─── LinkedIn Page Provider ────────────────────────────────────
// Extends LinkedIn with org-level posting (companies/pages).
// Same OAuth credentials as LinkedIn but uses organization URNs.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct LinkedInPageProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl LinkedInPageProvider {
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
impl SocialProvider for LinkedInPageProvider {
    fn identifier(&self) -> &'static str {
        "linkedin-page"
    }

    fn name(&self) -> &'static str {
        "LinkedIn Page"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "openid".into(),
            "profile".into(),
            "email".into(),
            "w_member_social".into(),
            "rw_organization_admin".into(),
            "w_organization_social".into(),
            "r_organization_social".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        3000
    }

    fn is_between_steps(&self) -> bool { true }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Post to a LinkedIn Company Page you administer")
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

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
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

    /// List organizations the user can administer
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get("https://api.linkedin.com/v2/organizationalEntityAcls?q=roleAssignee&role=ADMINISTRATOR&projection=(elements*(organizationalTarget~(localizedName,vanityName,logoV2(original~:playableStreams))))")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let elements = json["elements"].as_array().map(|a| a.to_vec()).unwrap_or_default();

        let pages = elements.iter().map(|e| {
            let target = &e["organizationalTarget~"];
            let id = e["organizationalTarget"].as_str()
                .unwrap_or("")
                .split(':')
                .next_back()
                .unwrap_or("")
                .to_string();
            PageInfo {
                id,
                name: target["localizedName"].as_str().unwrap_or("").to_string(),
                access_token: Some(access_token.to_string()),
                picture: target["logoV2"]["original~"]["elements"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|el| el["identifiers"].as_array())
                    .and_then(|ids| ids.first())
                    .and_then(|id| id["identifier"].as_str())
                    .map(String::from),
                username: target["vanityName"].as_str().map(String::from),
            }
        }).collect();

        Ok(pages)
    }

    async fn fetch_page_info(&self, access_token: &str, page_id: &str) -> Result<PageInfo, ProviderError> {
        let resp = self
            .http
            .get(format!("https://api.linkedin.com/v2/organizations/{page_id}?projection=(id,localizedName,vanityName,logoV2(original~:playableStreams))"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        Ok(PageInfo {
            id: json["id"].as_str().unwrap_or("").to_string(),
            name: json["localizedName"].as_str().unwrap_or("").to_string(),
            access_token: Some(access_token.to_string()),
            picture: json["logoV2"]["original~"]["elements"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|el| el["identifiers"].as_array())
                .and_then(|ids| ids.first())
                .and_then(|id| id["identifier"].as_str())
                .map(String::from),
            username: json["vanityName"].as_str().map(String::from),
        })
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
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Resolve org ID — internal_id tells us which org to post as
        let org_id = self.resolve_org_id(access_token).await?;

        let body = serde_json::json!({
            "author": format!("urn:li:organization:{org_id}"),
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
                platform_post_url: None,
                status: "published".into(),
            });
        }

        let json: serde_json::Value = resp.json().await?;

        if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn Page publish failed")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    fn map_error(&self, body: &str, _status: u16) -> Option<String> {
        if body.contains("Unable to obtain activity") {
            Some("Unable to obtain activity. Please try again.".into())
        } else if body.contains("resource is forbidden") {
            Some("Resource is forbidden. Check your organization permissions.".into())
        } else {
            None
        }
    }
}

impl LinkedInPageProvider {
    async fn resolve_org_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let pages = self.pages(access_token).await?;
        pages.first()
            .map(|p| p.id.clone())
            .ok_or_else(|| ProviderError::Auth("No LinkedIn organizations found".into()))
    }
}
