// ─── Telegram User Provider ─────────────────────────────────────
// Personal Telegram account via telegram-cli daemon.
// Unlike TelegramBotProvider (which uses Bot API with token),
// this uses the user-client CLI binary for full user account access.
// Based on WhatsAppProvider pattern.

use async_trait::async_trait;

use super::*;
use crate::config::Config;
use crate::services::telegram_daemon::TelegramDaemon;

/// Telegram User Account provider via telegram-cli daemon.
///
/// This provider uses the native Telegram CLI (telegram-cli / telegram-daemon)
/// to send messages from a personal Telegram user account, rather than
/// a bot account via the Bot API.
///
/// Unlike TelegramProvider (Bot API), this gives full user account access:
/// - Send to any peer (username, phone, group)
/// - Access to dialogs, contacts, search
/// - Real user account, not a bot
///
/// Authentication: The daemon must already be authenticated (phone + verification
/// code via telegram-cli's interactive mode). Check auth status via exchange_code.
pub struct TelegramUserProvider;

impl TelegramUserProvider {
    pub fn new(_config: &Config) -> Self {
        Self
    }

    /// Start a fresh telegram-cli daemon.
    /// Each call spawns a new process — the caller owns it until it goes out of scope.
    fn start_daemon() -> Result<Box<TelegramDaemon>, ProviderError> {
        TelegramDaemon::start().map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("binary not found") {
                ProviderError::Api(
                    "telegram-cli binary not found. Install telegram-cli and ensure \
                     tg/bin/telegram-cli exists, or add it to your PATH."
                        .into(),
                )
            } else {
                ProviderError::Api(format!("Failed to start Telegram daemon: {e}"))
            }
        })
    }
}

#[async_trait]
impl SocialProvider for TelegramUserProvider {
    fn identifier(&self) -> &'static str {
        "telegram-user"
    }

    fn name(&self) -> &'static str {
        "Telegram User"
    }

    fn tooltip(&self) -> Option<&'static str> {
        Some("Send messages from your personal Telegram account via CLI daemon")
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
        // CLI-based auth — no OAuth URL
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
        let daemon = Self::start_daemon()?;

        // Get self info to verify authentication
        let self_info = daemon
            .auth_status()
            .map_err(|e| ProviderError::Api(format!("Telegram auth check failed: {e}")))?;

        // telegram-cli returns user info if authenticated, or an error if not logged in.
        // If "self" field is present and has a name/phone, we're authenticated.
        let is_authenticated = self_info
            .get("print_name")
            .or_else(|| self_info.get("first_name"))
            .or_else(|| self_info.get("phone"))
            .is_some();

        if !is_authenticated {
            // Check for common error messages indicating not logged in
            let has_self = self_info.get("self").is_some();
            if has_self {
                // Self info exists but may be incomplete — still consider authenticated
                let name = self_info["first_name"]
                    .as_str()
                    .unwrap_or("Telegram User")
                    .to_string();
                let user_id = self_info["id"]
                    .as_i64()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                return Ok(AuthToken {
                    access_token: user_id.clone(),
                    refresh_token: None,
                    expires_in: Some(999_999_999),
                    provider_user_id: user_id,
                    name,
                    username: self_info["username"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    picture: None,
                });
            }

            return Err(ProviderError::Auth(
                "Telegram CLI not authenticated. Use telegram-cli interactive mode \
                 to connect your account first (phone + verification code)."
                    .into(),
            ));
        }

        // User is authenticated — extract identity
        let name = self_info["print_name"]
            .as_str()
            .or_else(|| self_info["first_name"].as_str())
            .unwrap_or("Telegram User")
            .to_string();

        let user_id = self_info["id"]
            .as_i64()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(AuthToken {
            access_token: user_id.clone(),
            refresh_token: None,
            expires_in: Some(999_999_999),
            provider_user_id: user_id,
            name,
            username: self_info["username"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            picture: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Telegram user tokens do not expire".into(),
        ))
    }

    async fn publish(
        &self,
        _access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // For CLI-based providers, we need the target peer from the post settings.
        // The target is stored in post.settings.target or similar.
        let target = post
            .settings
            .get("target")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                ProviderError::Api(
                    "No target peer specified. Add 'target' field in post settings \
                     (e.g., username, phone number, or chat name)."
                        .into(),
                )
            })?;

        let daemon = Self::start_daemon()?;

        // Text-only posting (media not yet supported via CLI)
        if post.media.is_empty() {
            let result = daemon
                .send_message(target, &post.content)
                .map_err(|e| ProviderError::Api(format!("Telegram send failed: {e}")))?;

            let msg_id = result
                .get("id")
                .or_else(|| result.get("message_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            Ok(PublishResult {
                platform_post_id: msg_id.to_string(),
                platform_post_url: None, // User accounts don't have public URLs
                status: "published".into(),
            })
        } else {
            Err(ProviderError::Api(
                "Media posting not yet supported for Telegram user accounts via CLI. \
                 Text messages are supported."
                    .into(),
            ))
        }
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Telegram user accounts do not support page management via CLI. \
             Use dialog_list to browse conversations."
                .into(),
        ))
    }
}