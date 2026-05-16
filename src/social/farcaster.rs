// ─── Farcaster Provider ──────────────────────────────────────
// Uses Neynar API to post casts. Non-OAuth (Web3 wallet auth).
// Stores signer_uuid as access_token.

use async_trait::async_trait;
use reqwest::StatusCode;

use super::*;
use crate::config::Config;

const NEYNAR_API_BASE: &str = "https://api.neynar.com/v2";

pub struct FarcasterProvider {
    client: reqwest::Client,
    neynar_api_key: Option<String>,
}

impl FarcasterProvider {
    pub fn new(config: &Config) -> Self {
        Self {
            client: reqwest::Client::new(),
            neynar_api_key: config.neynar_api_key.clone(),
        }
    }
}

#[async_trait]
impl SocialProvider for FarcasterProvider {
    fn identifier(&self) -> &'static str {
        "farcaster"
    }

    fn name(&self) -> &'static str {
        "Farcaster"
    }

    fn scopes(&self) -> Vec<String> {
        vec![]
    }

    fn max_content_length(&self) -> usize {
        320
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Normal
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Connect your Farcaster wallet. Uses Neynar API for publishing.")
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        Err(ProviderError::Auth(
            "Farcaster uses Web3 wallet authentication instead of OAuth. \
             Provide your wallet address or signer UUID directly."
                .into(),
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        if code.is_empty() {
            return Err(ProviderError::Auth(
                "Farcaster: provide your signer UUID via the Web3 connect API. \
                 Use POST /api/integrations/connect/web3 with \
                 {\"provider\":\"farcaster\",\"address\":\"<signer_uuid>\"}."
                    .into(),
            ));
        }

        // Parse JSON credentials (WordPress-style per-user credential storage)
        let creds: serde_json::Value = serde_json::from_str(code).map_err(|_| {
            ProviderError::Auth(
                "Invalid credentials format. Expected JSON with \"address\" field.".into(),
            )
        })?;

        let address = creds["address"]
            .as_str()
            .ok_or_else(|| {
                ProviderError::Auth("Missing 'address' in credentials".into())
            })?;

        if address.is_empty() {
            return Err(ProviderError::Auth("address must not be empty".into()));
        }

        let display = if address.len() > 10 {
            format!("{}...{}", &address[..6], &address[address.len() - 4..])
        } else {
            address.to_string()
        };

        Ok(AuthToken {
            access_token: address.to_string(),
            refresh_token: None,
            expires_in: None,
            provider_user_id: address.to_string(),
            name: format!("Farcaster ({})", display),
            username: address.to_string(),
            picture: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Farcaster tokens do not expire. Re-connect if needed.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let api_key = self.neynar_api_key.as_deref().ok_or_else(|| {
            ProviderError::Api(
                "NEYNAR_API_KEY not configured. Set it in your .env file.".into(),
            )
        })?;

        let signer_uuid = access_token;

        let body = serde_json::json!({
            "signer_uuid": signer_uuid,
            "text": post.content,
        });

        let resp = self
            .client
            .post(format!("{NEYNAR_API_BASE}/farcaster/cast"))
            .header("api_key", api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let response_body = resp.text().await.unwrap_or_default();
        let json: serde_json::Value =
            serde_json::from_str(&response_body).unwrap_or(serde_json::Value::Null);

        if status == StatusCode::OK || status == StatusCode::CREATED {
            let cast_hash = json["cast"]["hash"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let cast_url = format!("https://warpcast.com/{}", cast_hash);

            Ok(PublishResult {
                platform_post_id: cast_hash,
                platform_post_url: Some(cast_url),
                status: "published".into(),
            })
        } else {
            // Check for known error codes first
            if let Some(custom) = self.map_error(&response_body, status.as_u16()) {
                return Err(ProviderError::Api(custom));
            }
            let msg = json["message"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("Farcaster publish failed")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let display = if access_token.len() > 10 {
            format!("{}...{}", &access_token[..6], &access_token[access_token.len() - 4..])
        } else {
            access_token.to_string()
        };

        Ok(vec![PageInfo {
            id: access_token.to_string(),
            name: format!("Farcaster ({})", display),
            access_token: Some(access_token.to_string()),
            picture: None,
            username: Some(access_token.to_string()),
        }])
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let display = if access_token.len() > 10 {
            format!("{}...{}", &access_token[..6], &access_token[access_token.len() - 4..])
        } else {
            access_token.to_string()
        };

        Ok(PageInfo {
            id: access_token.to_string(),
            name: format!("Farcaster ({})", display),
            access_token: Some(access_token.to_string()),
            picture: None,
            username: Some(access_token.to_string()),
        })
    }

    fn map_error(&self, body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Invalid Neynar API key. Check NEYNAR_API_KEY in .env.".into())
        } else if status == 429 {
            Some("Farcaster/Neynar API rate limit exceeded. Try again later.".into())
        } else if body.contains("invalid signer") {
            Some("Invalid Farcaster signer. Re-connect your account.".into())
        } else {
            None
        }
    }
}
