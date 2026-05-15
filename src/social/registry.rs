// ─── Provider Registry ────────────────────────────────────────
// Central registry of all available social media providers.
// Used by both the API layer and the MCP layer to route requests.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use super::mastodon;
use super::slack;
use crate::config::Config;
use crate::services::telegram_client::TelegramClientManager;
use crate::wa::WhaClient;

/// Thread-safe provider registry
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Arc<HashMap<&'static str, Arc<dyn SocialProvider>>>,
}

impl ProviderRegistry {
    /// Build registry with all providers, given app config for credentials
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &Config,
        telegram_client_manager: Option<Arc<TelegramClientManager>>,
        wa_client: Option<Arc<tokio::sync::Mutex<WhaClient>>>,
    ) -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn SocialProvider>> = HashMap::new();

        // Current providers
        providers.insert("x", Arc::new(x::XProvider::new(config)));
        providers.insert(
            "linkedin",
            Arc::new(linkedin::LinkedInProvider::new(config)),
        );
        providers.insert("bluesky", Arc::new(bluesky::BlueskyProvider::new(config)));
        providers.insert(
            "facebook",
            Arc::new(facebook::FacebookProvider::new(config)),
        );
        providers.insert(
            "instagram",
            Arc::new(instagram::InstagramProvider::new(config)),
        );

        // New providers (with credentials)
        let linkedin_page = linkedin_page::LinkedInPageProvider::new(config);
        // Only add if credential check passes — LinkedIn page uses same credentials as LinkedIn
        if config.linkedin_client_id.is_some() {
            providers.insert("linkedin-page", Arc::new(linkedin_page));
        }

        if config.instagram_app_id.is_some() {
            providers.insert(
                "instagram-standalone",
                Arc::new(instagram_standalone::InstagramStandaloneProvider::new(
                    config,
                )),
            );
        }

        if config.threads_app_id.is_some() {
            providers.insert("threads", Arc::new(threads::ThreadsProvider::new(config)));
        }

        if config.youtube_client_id.is_some() {
            providers.insert("youtube", Arc::new(youtube::YoutubeProvider::new(config)));
        }

        // Always registered (show on frontend even without credentials)
        providers.insert("reddit", Arc::new(reddit::RedditProvider::new(config)));

        if config.discord_client_id.is_some() {
            providers.insert("discord", Arc::new(discord::DiscordProvider::new(config)));
        }

        // Telegram Bot — token-based accounts (comma-separated TELEGRAM_BOT_TOKENS)
        if config.telegram_bot_tokens.is_some() {
            providers.insert(
                "telegram-bot",
                Arc::new(telegram_bot::TelegramBotProvider::new(config)),
            );
        }

        // Telegram User — Grammers-based MTProto client (always registered)
        providers.insert(
            "telegram-user",
            Arc::new(telegram_user::TelegramUserProvider::new(config, telegram_client_manager.clone())),
        );

        // Always registered (show on frontend even without credentials)
        providers.insert("pinterest", Arc::new(pinterest::PinterestProvider::new(config)));

        // WhatsApp — native wa-rs client with wacli fallback
        providers.insert("whatsapp", Arc::new(whatsapp::WhatsAppProvider::new(config, wa_client.clone())));

        // TikTok — OAuth-based video platform
        if config.tiktok_client_id.is_some() {
            providers.insert("tiktok", Arc::new(tiktok::TikTokProvider::new(config)));
        }

        // Mastodon — OAuth-based microblogging (with app registration)
        if config.mastodon_client_id.is_some() {
            providers.insert("mastodon", Arc::new(mastodon::MastodonProvider::new(config)));
        }

        // Medium — API key-based publishing
        if config.medium_access_token.is_some() {
            providers.insert("medium", Arc::new(medium::MediumProvider::new(config)));
        }

        // Dev.to — API key-based publishing
        if config.devto_api_key.is_some() {
            providers.insert("devto", Arc::new(devto::DevtoProvider::new(config)));
        }

        // Hashnode — API key-based blogging
        if config.hashnode_api_key.is_some() {
            providers.insert("hashnode", Arc::new(hashnode::HashnodeProvider::new(config)));
        }

        // Chrome extension-based provider (no OAuth credentials needed)
        providers.insert("skool", Arc::new(skool::SkoolProvider::new()));

        // WordPress — REST API + Application Password (always registered, no global credentials)
        providers.insert("wordpress", Arc::new(wordpress::WordPressProvider::new(config)));

        // Slack — OAuth-based messaging workspace provider
        if config.slack_client_id.is_some() {
            providers.insert("slack", Arc::new(slack::SlackProvider::new(config)));
        }

        tracing::info!(
            "Provider registry initialized with: {}",
            providers.keys().cloned().collect::<Vec<_>>().join(", ")
        );

        Self {
            providers: Arc::new(providers),
        }
    }

    /// Get a provider by identifier
    pub fn get(&self, identifier: &str) -> Option<Arc<dyn SocialProvider>> {
        self.providers.get(identifier).cloned()
    }

    /// List all registered provider identifiers
    pub fn list(&self) -> Vec<&'static str> {
        self.providers.keys().copied().collect()
    }

    /// Get all providers
    pub fn all(&self) -> Vec<Arc<dyn SocialProvider>> {
        self.providers.values().cloned().collect()
    }
}
