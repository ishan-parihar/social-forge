use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::*;
use crate::config::Config;
use crate::services::whatsapp_daemon::WhatsAppDaemon;
use crate::wa::WhaClient;
use crate::wa::chats::{list_contacts, list_chats};

#[allow(dead_code)]
pub struct WhatsAppProvider {
    daemon: Arc<WhatsAppDaemon>,
    store_dir: PathBuf,
    wa_client: Option<Arc<Mutex<WhaClient>>>,
}

impl WhatsAppProvider {
    pub fn new(
        config: &Config,
        wa_client: Option<Arc<Mutex<WhaClient>>>,
    ) -> Self {
        let store_dir = config
            .whatsapp_store_dir
            .clone()
            .unwrap_or_else(|| "./data/whatsapp".into());
        let store_dir = PathBuf::from(&store_dir);

        // Only start daemon if binary is configured/found
        let daemon = match WhatsAppDaemon::start(store_dir.clone()) {
            Ok(d) => d,
            Err(_) => {
                // Create a dummy daemon
                let dummy = WhatsAppDaemon::new(
                    PathBuf::from("/usr/bin/true"),
                    store_dir.clone(),
                );
                Arc::new(dummy)
            }
        };

        Self { daemon, store_dir, wa_client }
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
        // Try native WhaClient first
        if let Some(ref wa_client) = self.wa_client {
            let locked = wa_client.lock().await;
            if locked.is_authenticated() {
                let jid = locked.inner().get_pn().await
                    .map(|j| j.to_string())
                    .unwrap_or_else(|| "unknown".into());
                return Ok(AuthToken {
                    access_token: jid.clone(),
                    refresh_token: None,
                    expires_in: Some(999_999_999),
                    provider_user_id: jid.clone(),
                    name: format!("WhatsApp ({jid})"),
                    username: jid,
                    picture: None,
                });
            }
            return Err(ProviderError::Auth(
                "WhatsApp not authenticated. Use wa_auth_status / wa_pair_code tools to link your device.".into(),
            ));
        }

        // Fallback: legacy daemon (wacli Go sidecar)
        let status = self
            .daemon
            .auth_status()
            .map_err(|e| ProviderError::Api(format!("WhatsApp auth check failed: {e}")))?;

        let authenticated = status["authenticated"].as_bool().unwrap_or(false);
        if !authenticated {
            return Err(ProviderError::Auth(
                "WhatsApp not authenticated. Use wa_auth_status / wa_pair_code tools to link your device.".into(),
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

        if !post.media.is_empty() {
            return Err(ProviderError::Api(
                "WhatsApp media posting not yet supported via server mode".into(),
            ));
        }

        // Try native WhaClient first
        if let Some(ref wa_client) = self.wa_client {
            let jid = wa_rs::Jid::pn(to);
            let msg_id = crate::wa::messages::send_text(wa_client, &jid, &post.content)
                .await
                .map_err(|e| ProviderError::Api(format!("WhatsApp send failed: {e}")))?;

            return Ok(PublishResult {
                platform_post_id: msg_id,
                platform_post_url: None,
                status: "published".into(),
            });
        }

        // Fallback: legacy daemon (wacli Go sidecar)
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

    async fn targets(&self, _access_token: &str) -> Result<Vec<TargetInfo>, ProviderError> {
        let Some(ref wa_client) = self.wa_client else {
            return Ok(vec![]);
        };

        {
            let locked = wa_client.lock().await;
            if !locked.is_authenticated() {
                return Ok(vec![]);
            }
        }

        let mut targets: Vec<TargetInfo> = Vec::new();

        if let Ok(contacts) = list_contacts(wa_client, Some(500)).await {
            for contact in contacts {
                let display_name = if !contact.name.is_empty() {
                    contact.name
                } else if !contact.push_name.is_empty() {
                    contact.push_name.clone()
                } else {
                    contact.jid.split('@').next().unwrap_or(&contact.jid).to_string()
                };

                targets.push(TargetInfo {
                    id: contact.jid,
                    name: display_name,
                    target_type: "contact".into(),
                    picture: None,
                    metadata: None,
                });
            }
        }

        if let Ok(chats) = list_chats(wa_client, Some(500)).await {
            for chat in chats {
                if chat.jid.ends_with("@g.us") {
                    let name = if chat.name.is_empty() {
                        chat.jid.split('@').next().unwrap_or(&chat.jid).to_string()
                    } else {
                        chat.name
                    };
                    targets.push(TargetInfo {
                        id: chat.jid,
                        name,
                        target_type: "group".into(),
                        picture: None,
                        metadata: None,
                    });
                }
            }
        }

        Ok(targets)
    }
}
