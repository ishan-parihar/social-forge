// ─── Provider Registry ────────────────────────────────────────
// Central registry of all available social media providers.
// Used by both the API layer and the MCP layer to route requests.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::config::Config;

/// Thread-safe provider registry
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Arc<HashMap<&'static str, Arc<dyn SocialProvider>>>,
}

impl ProviderRegistry {
    /// Build registry with all providers, given app config for credentials
    pub fn new(config: &Config) -> Self {
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

        if config.threads_client_id.is_some() {
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

        if config.telegram_token.is_some() {
            providers.insert(
                "telegram",
                Arc::new(telegram::TelegramProvider::new(config)),
            );
        }

        // Always registered (show on frontend even without credentials)
        providers.insert("pinterest", Arc::new(pinterest::PinterestProvider::new(config)));

        // Chrome extension-based provider (no OAuth credentials needed)
        providers.insert("skool", Arc::new(skool::SkoolProvider::new()));

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
