use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use super::*;
use crate::config::Config;
use crate::services::whatsapp_daemon::WhatsAppDaemon;

#[allow(dead_code)]
pub struct WhatsAppProvider {
    daemon: Arc<WhatsAppDaemon>,
    store_dir: PathBuf,
}

impl WhatsAppProvider {
    pub fn new(config: &Config) -> Self {
        let store_dir = config
            .whatsapp_store_dir
            .clone()
            .unwrap_or_else(|| "./data/whatsapp".into());
        let store_dir = PathBuf::from(&store_dir);

        // Only start daemon if binary is configured/found
        let daemon = match WhatsAppDaemon::start(store_dir.clone()) {
            Ok(d) => d,
            Err(_) => {
                tracing::warn!(
                    "WhatsApp daemon not available. Install wacli and run scripts/build-wacli.sh"
                );
                // Create a dummy daemon by starting with dummy binary
                let dummy = WhatsAppDaemon::new(
                    PathBuf::from("/usr/bin/true"),
                    store_dir.clone(),
                );
                Arc::new(dummy)
            }
        };

        Self { daemon, store_dir }
    }
}

#[async_trait]
impl SocialProvider for WhatsAppProvider {
    fn identifier(&self) -> &'static str {
        "whatsapp"
    }

    fn name(&self) -> &'static str {
        "WhatsApp"
    }

    fn scopes(&self) -> Vec<String> {
        vec![]
    }

    fn max_content_length(&self) -> usize {
        4096
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    fn one_time_token(&self) -> bool {
        true
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Normal
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
        _code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        let status = self
            .daemon
            .auth_status()
            .map_err(|e| ProviderError::Api(format!("WhatsApp auth check failed: {e}")))?;

        let authenticated = status["authenticated"].as_bool().unwrap_or(false);
        if !authenticated {
            return Err(ProviderError::Auth(
                "WhatsApp not authenticated. Run: wacli auth --store <store_dir>".into(),
            ));
        }

        let jid = status["jid"].as_str().unwrap_or("unknown");
        Ok(AuthToken {
            access_token: jid.to_string(),
            refresh_token: None,
            expires_in: Some(999_999_999),
            provider_user_id: jid.to_string(),
            name: format!("WhatsApp ({jid})"),
            username: jid.to_string(),
            picture: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "WhatsApp tokens do not expire".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let to = access_token;

        if post.media.is_empty() {
            let result = self
                .daemon
                .send_text(to, &post.content)
                .map_err(|e| ProviderError::Api(format!("WhatsApp send failed: {e}")))?;

            let msg_id = result["message_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            Ok(PublishResult {
                platform_post_id: msg_id,
                platform_post_url: None,
                status: "published".into(),
            })
        } else {
            Err(ProviderError::Api(
                "WhatsApp media posting not yet supported via server mode".into(),
            ))
        }
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "WhatsApp does not support page management".into(),
        ))
    }
}
