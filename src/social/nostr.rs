// ─── Nostr Provider ──────────────────────────────────────────
// Uses Nostr protocol with npub authentication (non-OAuth).
// Stores npub as access_token. Publishing uses configured Nostr key.

use async_trait::async_trait;
use uuid::Uuid;

use super::*;
use crate::config::Config;

pub struct NostrProvider {
    #[expect(dead_code, reason = "reserved for future WebSocket relay connections")]
    http: reqwest::Client,
    nostr_private_key: Option<String>,
}

impl NostrProvider {
    pub fn new(config: &Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            nostr_private_key: config.nostr_private_key.clone(),
        }
    }
}

#[async_trait]
impl SocialProvider for NostrProvider {
    fn identifier(&self) -> &'static str {
        "nostr"
    }

    fn name(&self) -> &'static str {
        "Nostr"
    }

    fn scopes(&self) -> Vec<String> {
        vec![]
    }

    fn max_content_length(&self) -> usize {
        50000
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Normal
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Connect your Nostr npub. Publishing uses a configured Nostr key.")
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        Err(ProviderError::Auth(
            "Nostr uses npub authentication instead of OAuth. \
             Provide your npub directly."
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
                "Nostr: provide your npub via the Web3 connect API. \
                 Use POST /api/integrations/connect/web3 with \
                 {\"provider\":\"nostr\",\"address\":\"<npub>\"}."
                    .into(),
            ));
        }

        // Parse JSON credentials
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

        if !address.starts_with("npub1") {
            return Err(ProviderError::Auth(
                "Invalid Nostr public key format. Expected npub1...".into(),
            ));
        }

        let display = if address.len() > 12 {
            format!("{}...{}", &address[..8], &address[address.len() - 4..])
        } else {
            address.to_string()
        };

        Ok(AuthToken {
            access_token: address.to_string(),
            refresh_token: None,
            expires_in: None,
            provider_user_id: address.to_string(),
            name: format!("Nostr ({})", display),
            username: address.to_string(),
            picture: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Nostr npub does not expire. Re-connect if needed.".into(),
        ))
    }

    async fn publish(
        &self,
        _access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Generate a unique placeholder event ID
        let event_id = format!("nostr_{}", Uuid::new_v4());

        // If a Nostr private key is configured, use it for signing and publishing
        if let Some(_private_key) = &self.nostr_private_key {
            // TODO: Implement actual Nostr event signing and relay publishing.
            // Requires the nostr crate for event creation, signing (Schnorr),
            // and WebSocket relay communication.
            //
            // Steps to implement:
            // 1. Parse the private key (nsec or hex)
            // 2. Create a kind 1 (text note) event
            // 3. Sign the event with the private key
            // 4. POST to relay(s) via WebSocket
            // 5. Return the event ID and note URL
            tracing::info!(
                "Nostr publish would use configured key with {} bytes of content",
                post.content.len()
            );
        }

        // Return placeholder result — actual relay publishing requires
        // event signing which needs the user's private key or a configured
        // Nostr relay signing key.
        Ok(PublishResult {
            platform_post_id: event_id,
            platform_post_url: None,
            status: "pending_implementation".into(),
        })
    }

    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let display = if access_token.len() > 12 {
            format!("{}...{}", &access_token[..8], &access_token[access_token.len() - 4..])
        } else {
            access_token.to_string()
        };

        Ok(vec![PageInfo {
            id: access_token.to_string(),
            name: format!("Nostr ({})", display),
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
        let display = if access_token.len() > 12 {
            format!("{}...{}", &access_token[..8], &access_token[access_token.len() - 4..])
        } else {
            access_token.to_string()
        };

        Ok(PageInfo {
            id: access_token.to_string(),
            name: format!("Nostr ({})", display),
            access_token: Some(access_token.to_string()),
            picture: None,
            username: Some(access_token.to_string()),
        })
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Invalid Nostr credentials.".into())
        } else if status == 429 {
            Some("Nostr relay rate limit exceeded. Try again later.".into())
        } else {
            None
        }
    }
}
