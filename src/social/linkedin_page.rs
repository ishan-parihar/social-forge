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
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        let id_str = json["id"].as_u64()
            .map(|n| n.to_string())
            .or_else(|| json["id"].as_str().map(String::from))
            .unwrap_or_else(|| page_id.to_string());

        Ok(PageInfo {
            id: id_str,
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

    async fn analytics(
        &self,
        access_token: &str,
        internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let org_id = if internal_id.is_empty() {
            self.resolve_org_id(access_token).await?
        } else {
            internal_id.to_string()
        };
        let org_urn = format!("urn:li:organization:{org_id}");

        let mut results = Vec::new();

        // Share statistics
        let share_url = format!(
            "https://api.linkedin.com/rest/organizationalEntityShareStatistics?q=organizationalEntity&organizationalEntity={org_urn}"
        );
        let resp = self
            .http
            .get(&share_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202601")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("LinkedIn API rate limit".into()));
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if !status.is_success() {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn share statistics error")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                if let Some(stats) = element["totalShareStatistics"].as_object() {
                    for (key, val) in stats {
                        if let Some(n) = val.as_u64() {
                            results.push(AnalyticsData {
                                label: key.clone(),
                                data: vec![AnalyticsDataPoint {
                                    total: n.to_string(),
                                    date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                }],
                                percentage_change: 0.0,
                            });
                        }
                    }
                }
            }
        }

        // Follower count
        let follower_url = format!(
            "https://api.linkedin.com/rest/networkSizes/{org_urn}?edgeType=CompanyFollowedByMember"
        );
        let resp = self
            .http
            .get(&follower_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202601")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("LinkedIn API rate limit".into()));
        }

        if status.is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(elements) = json["elements"].as_array() {
                if let Some(elem) = elements.first() {
                    if let Some(count) = elem["firstDegreeSize"].as_u64() {
                        results.push(AnalyticsData {
                            label: "followerCount".into(),
                            data: vec![AnalyticsDataPoint {
                                total: count.to_string(),
                                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                            }],
                            percentage_change: 0.0,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let url = format!(
            "https://api.linkedin.com/rest/shares/{platform_post_id}/shareStatistics"
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("LinkedIn-Version", "202601")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("LinkedIn API rate limit".into()));
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if !status.is_success() {
            let msg = json["message"]
                .as_str()
                .unwrap_or("LinkedIn post statistics error")
                .to_string();
            return Err(ProviderError::Api(msg));
        }

        let mut results = Vec::new();

        if let Some(elements) = json["elements"].as_array() {
            for element in elements {
                if let Some(stats) = element["totalShareStatistics"].as_object() {
                    for (key, val) in stats {
                        if let Some(n) = val.as_u64() {
                            results.push(AnalyticsData {
                                label: key.clone(),
                                data: vec![AnalyticsDataPoint {
                                    total: n.to_string(),
                                    date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                }],
                                percentage_change: 0.0,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

impl LinkedInPageProvider {
    async fn resolve_org_id(&self, access_token: &str) -> Result<String, ProviderError> {
        let pages = self.pages(access_token).await?;
        pages.first()
            .map(|p| p.id.clone())
            .ok_or_else(|| ProviderError::Auth("No LinkedIn organizations found".into()))
    }

    pub async fn get_page_posts(&self, access_token: &str, page_id: &str, limit: u32) -> Result<serde_json::Value, ProviderError> {
        let limit = limit.clamp(1, 100);
        let url = format!("https://api.linkedin.com/v2/rest/posts?author=urn:li:organization:{page_id}&count={limit}");
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

        if status == 200 {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["message"].as_str().unwrap_or("LinkedIn Page API error").to_string();
            Err(ProviderError::Api(msg))
        }
    }

    pub async fn create_comment(&self, access_token: &str, post_urn: &str, page_urn: &str, message: &str) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://api.linkedin.com/v2/rest/socialActions/{post_urn}/comments");
        let body = serde_json::json!({
            "actor": page_urn,
            "message": { "text": message },
            "object": post_urn
        });
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", "202601")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == 201 {
            let json: serde_json::Value = resp.json().await?;
            Ok(json)
        } else {
            let json: serde_json::Value = resp.json().await?;
            if status == 401 {
                Err(ProviderError::TokenExpired)
            } else {
                let msg = json["message"].as_str().unwrap_or("LinkedIn Page API error").to_string();
                Err(ProviderError::Api(msg))
            }
        }
    }
}
